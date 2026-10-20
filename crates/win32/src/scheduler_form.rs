//! Native controls and signal-only callbacks for the modeless scheduler.
use super::{Tab, UiAction};
use crate::{
    commands::*,
    controls::scale,
    platform::{self, item, resource, send, set_text, text, wide},
    presenter::{self, ViewModel},
    shell_icons::ShellImages,
    AppError,
};
use rusty_engine::{Date, JobId, JobState, LocalStamp, QueueId, QueueSettings, StartRule};
use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
};
use windows::{
    core::{w, PWSTR},
    Win32::{
        Foundation::{HWND, LPARAM, SYSTEMTIME, WPARAM},
        UI::{
            Controls::*, HiDpi::GetDpiForWindow, Input::KeyboardAndMouse::EnableWindow,
            WindowsAndMessaging::*,
        },
    },
};
const SCHEDULE_FIELDS: [i32; 18] = [
    SCHED_STARTUP,
    SCHED_ENABLED,
    SCHED_TIME,
    SCHED_ONCE,
    SCHED_DATE,
    SCHED_WEEKLY,
    SCHED_SUN,
    SCHED_MON,
    SCHED_TUE,
    SCHED_WED,
    SCHED_THU,
    SCHED_FRI,
    SCHED_SAT,
    SCHED_STOP_ENABLED,
    SCHED_STOP_TIME,
    SCHED_HINT,
    SCHED_NEXT,
    SCHED_NAME,
];
const FILE_FIELDS: [i32; 8] = [
    SCHED_PARALLEL,
    SCHED_SPIN,
    SCHED_PARALLEL_LABEL,
    SCHED_PARALLEL_TAIL,
    SCHED_FILES,
    SCHED_UP,
    SCHED_DOWN,
    SCHED_REMOVE,
];
#[derive(Default)]
struct Signals {
    hwnd: Cell<HWND>,
    actions: RefCell<VecDeque<UiAction>>,
    suppress: Cell<bool>,
    queues: RefCell<Vec<QueueId>>,
    files: RefCell<Vec<JobId>>,
}
impl Signals {
    fn push(&self, action: UiAction) {
        if self.suppress.get() {
            return;
        }
        let mut actions = self.actions.borrow_mut();
        if actions.len() < 128 {
            actions.push_back(action);
        }
    }
}
// Own the signal box before dialog construction so every error path destroys
// the HWND before its callback context can be freed.
struct SignalOwner(Box<Signals>);
impl std::ops::Deref for SignalOwner {
    type Target = Signals;
    fn deref(&self) -> &Signals {
        &self.0
    }
}
impl Drop for SignalOwner {
    fn drop(&mut self) {
        let hwnd = self.hwnd.get();
        if !hwnd.is_invalid() {
            unsafe {
                let _ = DestroyWindow(hwnd);
            }
        }
    }
}
struct Suppress<'a>(&'a Cell<bool>, bool);
impl<'a> Suppress<'a> {
    fn new(cell: &'a Cell<bool>) -> Self {
        Self(cell, cell.replace(true))
    }
}
impl Drop for Suppress<'_> {
    fn drop(&mut self) {
        self.0.set(self.1);
    }
}
pub(super) struct Form {
    signals: SignalOwner,
    shell: ShellImages,
    queue_rows: Vec<(QueueId, String, HTREEITEM)>,
    file_rows: Vec<(JobId, [String; 4])>,
    dpi: u32,
    tab: Tab,
}
impl Form {
    pub fn new(owner: HWND) -> Result<Self, AppError> {
        let signals = SignalOwner(Box::<Signals>::default());
        let hwnd = unsafe {
            CreateDialogParamW(
                Some(platform::instance()?),
                resource(SCHEDULER_DIALOG),
                Some(owner),
                Some(callback),
                LPARAM((&*signals as *const Signals) as isize),
            )
        }?;
        let tree = item(hwnd, SCHED_TREE)?;
        let list = item(hwnd, SCHED_FILES)?;
        let shell =
            match ShellImages::new(hwnd, tree, list, unsafe { GetDpiForWindow(hwnd).max(96) }) {
                Ok(shell) => shell,
                Err(error) => {
                    unsafe {
                        let _ = DestroyWindow(hwnd);
                    }
                    return Err(error);
                }
            };
        let mut form = Self {
            signals,
            shell,
            queue_rows: vec![],
            file_rows: vec![],
            dpi: unsafe { GetDpiForWindow(hwnd).max(96) },
            tab: Tab::Schedule,
        };
        let _guard = Suppress::new(&form.signals.suppress);
        let tabs = item(hwnd, SCHED_TABS)?;
        for (index, title) in ["Schedule", "Files in the queue"].iter().enumerate() {
            let mut title = wide(title);
            let tab = TCITEMW {
                mask: TCIF_TEXT,
                pszText: PWSTR(title.as_mut_ptr()),
                ..Default::default()
            };
            send(tabs, TCM_INSERTITEMW, index, &tab as *const _ as isize);
        }
        send(
            list,
            LVM_SETEXTENDEDLISTVIEWSTYLE,
            0,
            (LVS_EX_FULLROWSELECT | LVS_EX_DOUBLEBUFFER | LVS_EX_LABELTIP) as isize,
        );
        for (index, (title, width)) in [
            ("File Name", 275),
            ("Size", 80),
            ("Status", 125),
            ("Time left", 75),
        ]
        .iter()
        .enumerate()
        {
            let mut title = wide(title);
            let col = LVCOLUMNW {
                mask: LVCF_TEXT | LVCF_WIDTH | LVCF_SUBITEM,
                pszText: PWSTR(title.as_mut_ptr()),
                cx: scale(*width, form.dpi),
                iSubItem: index as i32,
                ..Default::default()
            };
            send(list, LVM_INSERTCOLUMNW, index, &col as *const _ as isize);
        }
        send(item(hwnd, SCHED_SPIN)?, UDM_SETRANGE32, 1, 2);
        for id in [SCHED_TIME, SCHED_STOP_TIME] {
            send(item(hwnd, id)?, DTM_SETFORMATW, 0, w!("HH':'mm").0 as isize);
        }
        if let Ok(icon) = platform::app_icon() {
            send(hwnd, WM_SETICON, ICON_SMALL as usize, icon.0 as isize);
            send(hwnd, WM_SETICON, ICON_BIG as usize, icon.0 as isize);
        }
        drop(_guard);
        form.page(Tab::Schedule);
        Ok(form)
    }
    pub fn hwnd(&self) -> HWND {
        self.signals.hwnd.get()
    }
    pub fn alive(&self) -> bool {
        !self.hwnd().is_invalid()
    }
    pub fn take_actions(&self) -> Vec<UiAction> {
        self.signals.actions.borrow_mut().drain(..).collect()
    }
    pub fn show(&self) {
        platform::bring_to_front(self.hwnd());
    }
    pub fn is_dialog_message(&self, msg: &MSG) -> bool {
        self.alive() && unsafe { IsDialogMessageW(self.hwnd(), msg).as_bool() }
    }
    pub fn page(&mut self, tab: Tab) {
        self.tab = tab;
        let _guard = Suppress::new(&self.signals.suppress);
        for id in SCHEDULE_FIELDS {
            visible(self.hwnd(), id, tab == Tab::Schedule || id == SCHED_NAME);
        }
        for id in FILE_FIELDS {
            visible(self.hwnd(), id, tab == Tab::Files);
        }
        if let Ok(tabs) = item(self.hwnd(), SCHED_TABS) {
            send(
                tabs,
                TCM_SETCURSEL,
                if tab == Tab::Schedule { 0 } else { 1 },
                0,
            );
        }
    }
    pub fn write(&self, settings: &QueueSettings) {
        let _guard = Suppress::new(&self.signals.suppress);
        let hwnd = self.hwnd();
        value(hwnd, SCHED_PARALLEL, &settings.parallelism.to_string());
        checked(hwnd, SCHED_STARTUP, settings.schedule.start_on_startup);
        checked(hwnd, SCHED_ENABLED, settings.schedule.enabled);
        let (once, date, minute, days) = match settings.schedule.start {
            StartRule::Once(at) => (true, at.date, at.minute, 127),
            StartRule::Weekly { minute, days } => (
                false,
                LocalStamp::now().map_or(
                    Date {
                        year: 2026,
                        month: 1,
                        day: 1,
                    },
                    |now| now.date,
                ),
                minute,
                days,
            ),
        };
        checked(hwnd, SCHED_ONCE, once);
        checked(hwnd, SCHED_WEEKLY, !once);
        set_date(hwnd, SCHED_DATE, date, minute);
        set_date(hwnd, SCHED_TIME, date, minute);
        for day in 0..7 {
            checked(hwnd, SCHED_SUN + day, days & (1 << day) != 0);
        }
        checked(
            hwnd,
            SCHED_STOP_ENABLED,
            settings.schedule.stop_minute.is_some(),
        );
        set_date(
            hwnd,
            SCHED_STOP_TIME,
            date,
            settings.schedule.stop_minute.unwrap_or(7 * 60),
        );
    }
    pub fn read(&self) -> Result<QueueSettings, String> {
        let hwnd = self.hwnd();
        let parallelism = item(hwnd, SCHED_PARALLEL)
            .map(text)
            .unwrap_or_default()
            .trim()
            .parse::<u8>()
            .ok()
            .filter(|n| (1..=2).contains(n))
            .ok_or("Enter one or two simultaneous downloads.")?;
        let time = get_date(hwnd, SCHED_TIME)?;
        let minute = time.wHour * 60 + time.wMinute;
        let start = if is_checked(hwnd, SCHED_ONCE) {
            let date = get_date(hwnd, SCHED_DATE)?;
            StartRule::Once(LocalStamp {
                date: Date {
                    year: date.wYear,
                    month: date.wMonth as u8,
                    day: date.wDay as u8,
                },
                minute,
            })
        } else {
            let mut days = 0;
            for day in 0..7 {
                if is_checked(hwnd, SCHED_SUN + day) {
                    days |= 1 << day;
                }
            }
            StartRule::Weekly { minute, days }
        };
        let stop_minute = if is_checked(hwnd, SCHED_STOP_ENABLED) {
            let time = get_date(hwnd, SCHED_STOP_TIME)?;
            Some(time.wHour * 60 + time.wMinute)
        } else {
            None
        };
        let settings = QueueSettings {
            parallelism,
            schedule: rusty_engine::QueueSchedule {
                start_on_startup: is_checked(hwnd, SCHED_STARTUP),
                enabled: is_checked(hwnd, SCHED_ENABLED),
                start,
                stop_minute,
            },
        };
        settings.schedule.validate()?;
        Ok(settings)
    }
    pub fn draw(
        &mut self,
        view: &ViewModel,
        selected: QueueId,
        dirty: bool,
        pending: bool,
    ) -> Result<(), AppError> {
        if !self.alive() {
            return Ok(());
        }
        let hwnd = self.hwnd();
        let _guard = Suppress::new(&self.signals.suppress);
        self.shell.drain();
        let names: Vec<_> = view
            .queues
            .values()
            .map(|q| (q.id, q.name.clone()))
            .collect();
        if self
            .queue_rows
            .iter()
            .map(|(id, name, _)| (*id, name.clone()))
            .collect::<Vec<_>>()
            != names
        {
            let tree = item(hwnd, SCHED_TREE)?;
            send(tree, WM_SETREDRAW, 0, 0);
            send(tree, TVM_DELETEITEM, 0, TVI_ROOT.0);
            self.queue_rows.clear();
            *self.signals.queues.borrow_mut() = names.iter().map(|(id, _)| *id).collect();
            for (index, (id, name)) in names.into_iter().enumerate() {
                let mut name_text = wide(&name);
                let node = TVINSERTSTRUCTW {
                    hParent: TVI_ROOT,
                    hInsertAfter: TVI_LAST,
                    Anonymous: TVINSERTSTRUCTW_0 {
                        item: TVITEMW {
                            mask: TVIF_TEXT | TVIF_PARAM,
                            pszText: PWSTR(name_text.as_mut_ptr()),
                            lParam: LPARAM(index as isize),
                            ..Default::default()
                        },
                    },
                };
                let handle =
                    HTREEITEM(send(tree, TVM_INSERTITEMW, 0, &node as *const _ as isize).0);
                self.queue_rows.push((id, name, handle));
            }
            send(tree, WM_SETREDRAW, 1, 0);
        }
        let tree = item(hwnd, SCHED_TREE)?;
        for (id, _, handle) in &self.queue_rows {
            let index = self.shell.index(ShellImages::stock(None));
            let node = TVITEMW {
                mask: TVIF_IMAGE | TVIF_SELECTEDIMAGE,
                hItem: *handle,
                iImage: index,
                iSelectedImage: index,
                ..Default::default()
            };
            send(tree, TVM_SETITEMW, 0, &node as *const _ as isize);
            if *id == selected {
                send(tree, TVM_SELECTITEM, TVGN_CARET as usize, handle.0);
            }
        }
        let saved_selection = selected_file(&self.signals);
        let list = item(hwnd, SCHED_FILES)?;
        let ids = view.visible_jobs(presenter::Category::Queue(selected));
        if ids != self.file_rows.iter().map(|(id, _)| *id).collect::<Vec<_>>() {
            send(list, WM_SETREDRAW, 0, 0);
            send(list, LVM_DELETEALLITEMS, 0, 0);
            self.file_rows.clear();
            *self.signals.files.borrow_mut() = ids.clone();
            for (row, id) in ids.iter().enumerate() {
                let job = &view.jobs[id];
                let mut label = wide(&job.filename);
                let item = LVITEMW {
                    mask: LVIF_TEXT | LVIF_PARAM,
                    iItem: row as i32,
                    pszText: PWSTR(label.as_mut_ptr()),
                    lParam: LPARAM(row as isize),
                    ..Default::default()
                };
                send(list, LVM_INSERTITEMW, 0, &item as *const _ as isize);
                self.file_rows.push((*id, Default::default()));
                if Some(*id) == saved_selection {
                    let item = LVITEMW {
                        state: LVIS_SELECTED | LVIS_FOCUSED,
                        stateMask: LVIS_SELECTED | LVIS_FOCUSED,
                        ..Default::default()
                    };
                    send(list, LVM_SETITEMSTATE, row, &item as *const _ as isize);
                }
            }
            send(list, WM_SETREDRAW, 1, 0);
        }
        for (row, (id, old)) in self.file_rows.iter_mut().enumerate() {
            let job = &view.jobs[id];
            let text = presenter::format_snapshot(job).columns;
            let cells = [
                text[0].clone(),
                text[1].clone(),
                text[2].clone(),
                text[3].clone(),
            ];
            for col in 0..4 {
                if old[col] != cells[col] {
                    let mut text = wide(&cells[col]);
                    let item = LVITEMW {
                        iSubItem: col as i32,
                        pszText: PWSTR(text.as_mut_ptr()),
                        ..Default::default()
                    };
                    send(list, LVM_SETITEMTEXTW, row, &item as *const _ as isize);
                }
            }
            *old = cells;
            let item = LVITEMW {
                mask: LVIF_IMAGE,
                iItem: row as i32,
                iImage: self.shell.index(ShellImages::file(job)),
                ..Default::default()
            };
            send(list, LVM_SETITEMW, 0, &item as *const _ as isize);
        }
        let queue = view.queues.get(&selected);
        value(
            hwnd,
            SCHED_NAME,
            queue.map_or("Queue no longer exists", |q| q.name.as_str()),
        );
        let status = queue
            .map(|q| {
                format!(
                    "{} | {} active, {} completed, {} entries{}",
                    if q.stopping {
                        "Stopping"
                    } else if q.running {
                        "Running"
                    } else {
                        "Held"
                    },
                    q.active,
                    q.completed,
                    q.entries.len(),
                    q.error
                        .as_ref()
                        .map(|e| format!(" | {e}"))
                        .unwrap_or_default()
                )
            })
            .unwrap_or_default();
        value(hwnd, SCHED_STATUS, &status);
        let next = queue
            .map(|q| {
                format!(
                    "Next start: {}{}",
                    q.schedule_status.next_start.map_or_else(
                        || if q.schedule.enabled {
                            "No future start; choose a new time".into()
                        } else {
                            "Not scheduled".into()
                        },
                        LocalStamp::label
                    ),
                    q.schedule_status
                        .stop_at
                        .map(|at| format!(" | Stop: {}", at.label()))
                        .unwrap_or_default()
                )
            })
            .unwrap_or_default();
        value(hwnd, SCHED_NEXT, &next);
        for id in [
            SCHED_STARTUP,
            SCHED_ENABLED,
            SCHED_PARALLEL,
            SCHED_SPIN,
            SCHED_STOP_ENABLED,
        ] {
            enable(hwnd, id, queue.is_some() && !pending);
        }
        let timed = is_checked(hwnd, SCHED_ENABLED) && !pending;
        let once = is_checked(hwnd, SCHED_ONCE);
        for id in [SCHED_TIME, SCHED_ONCE, SCHED_WEEKLY] {
            enable(hwnd, id, timed);
        }
        enable(hwnd, SCHED_DATE, timed && once);
        for id in SCHED_SUN..=SCHED_SAT {
            enable(hwnd, id, timed && !once);
        }
        enable(
            hwnd,
            SCHED_STOP_TIME,
            !pending && is_checked(hwnd, SCHED_STOP_ENABLED),
        );
        let file = selected_file(&self.signals).and_then(|id| view.jobs.get(&id));
        enable(hwnd, SCHED_UP, !pending && file.is_some());
        enable(hwnd, SCHED_DOWN, !pending && file.is_some());
        enable(
            hwnd,
            SCHED_REMOVE,
            !pending
                && file.is_some_and(|j| {
                    !matches!(
                        j.state,
                        JobState::Running | JobState::Pausing | JobState::Cancelling
                    )
                }),
        );
        enable(
            hwnd,
            SCHED_DELETE,
            !pending && queue.is_some_and(|q| q.id != QueueId::MAIN && q.entries.is_empty()),
        );
        enable(
            hwnd,
            SCHED_START,
            !pending && queue.is_some_and(|q| !q.running && !q.stopping),
        );
        enable(
            hwnd,
            SCHED_STOP,
            queue.is_some_and(|q| q.running || q.active > 0 || pending),
        );
        enable(hwnd, SCHED_APPLY, dirty && !pending && queue.is_some());
        enable(hwnd, SCHED_NEW, !pending);
        enable(hwnd, SCHED_TREE, !pending);
        Ok(())
    }
    pub fn invalidate_associations(&mut self) -> Result<(), AppError> {
        self.shell.invalidate(self.dpi)
    }
    pub fn dpi(&mut self, dpi: u32) -> Result<(), AppError> {
        self.shell.invalidate(dpi)?;
        if let Ok(list) = item(self.hwnd(), SCHED_FILES) {
            for (index, width) in [275, 80, 125, 75].iter().enumerate() {
                send(list, LVM_SETCOLUMNWIDTH, index, scale(*width, dpi) as isize);
            }
        }
        self.dpi = dpi;
        Ok(())
    }
}
impl Drop for Form {
    fn drop(&mut self) {
        let hwnd = self.hwnd();
        if !hwnd.is_invalid() {
            unsafe {
                let _ = DestroyWindow(hwnd);
            }
        }
    }
}
fn value(hwnd: HWND, id: i32, value: &str) {
    if let Ok(control) = item(hwnd, id) {
        if text(control) != value {
            set_text(control, value);
        }
    }
}
fn enable(hwnd: HWND, id: i32, on: bool) {
    if let Ok(control) = item(hwnd, id) {
        unsafe {
            let _ = EnableWindow(control, on);
        }
    }
}
fn visible(hwnd: HWND, id: i32, on: bool) {
    if let Ok(control) = item(hwnd, id) {
        unsafe {
            let _ = ShowWindow(control, if on { SW_SHOW } else { SW_HIDE });
        }
    }
}
fn checked(hwnd: HWND, id: i32, on: bool) {
    if let Ok(control) = item(hwnd, id) {
        send(
            control,
            BM_SETCHECK,
            if on { BST_CHECKED.0 } else { BST_UNCHECKED.0 } as usize,
            0,
        );
    }
}
fn is_checked(hwnd: HWND, id: i32) -> bool {
    item(hwnd, id).is_ok_and(|h| send(h, BM_GETCHECK, 0, 0).0 == BST_CHECKED.0 as isize)
}
fn set_date(hwnd: HWND, id: i32, date: Date, minute: u16) {
    if let Ok(control) = item(hwnd, id) {
        let time = SYSTEMTIME {
            wYear: date.year,
            wMonth: date.month as u16,
            wDay: date.day as u16,
            wHour: minute / 60,
            wMinute: minute % 60,
            ..Default::default()
        };
        send(
            control,
            DTM_SETSYSTEMTIME,
            GDT_VALID.0 as usize,
            &time as *const _ as isize,
        );
    }
}
fn get_date(hwnd: HWND, id: i32) -> Result<SYSTEMTIME, String> {
    let control = item(hwnd, id).map_err(|_| "Date/time control is unavailable.")?;
    let mut time = SYSTEMTIME::default();
    if send(control, DTM_GETSYSTEMTIME, 0, &mut time as *mut _ as isize).0 != GDT_VALID.0 as isize {
        return Err("Choose a valid date and time.".into());
    }
    Ok(time)
}
fn selected_file(signals: &Signals) -> Option<JobId> {
    let list = item(signals.hwnd.get(), SCHED_FILES).ok()?;
    let row = send(list, LVM_GETNEXTITEM, usize::MAX, LVNI_SELECTED as isize).0;
    if row < 0 {
        None
    } else {
        signals.files.borrow().get(row as usize).copied()
    }
}
unsafe extern "system" fn callback(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> isize {
    std::panic::catch_unwind(|| unsafe { inner(hwnd, msg, w, l) }).unwrap_or(0)
}
unsafe fn inner(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> isize {
    unsafe {
        if msg == WM_INITDIALOG {
            if l.0 == 0 {
                return 0;
            }
            let signals = &*(l.0 as *const Signals);
            signals.hwnd.set(hwnd);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, l.0);
            return 1;
        }
        let pointer = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Signals;
        if pointer.is_null() {
            return 0;
        }
        let signals = &*pointer;
        match msg {
            WM_COMMAND => {
                let id = (w.0 & 0xffff) as i32;
                let notification = (w.0 >> 16) as u32;
                if id == 1 {
                    signals.push(UiAction::Button(SCHED_APPLY, None));
                    return 1;
                }
                if id == 2 {
                    signals.push(UiAction::Button(SCHED_CLOSE, None));
                    return 1;
                }
                if (SCHED_NEW..=SCHED_REMOVE).contains(&id) {
                    signals.push(UiAction::Button(id, selected_file(signals)));
                    return 1;
                }
                if notification == BN_CLICKED || id == SCHED_PARALLEL && notification == EN_CHANGE {
                    signals.push(UiAction::Changed);
                }
            }
            WM_NOTIFY if l.0 != 0 => {
                let header = &*(l.0 as *const NMHDR);
                match (header.idFrom as i32, header.code) {
                    (SCHED_TREE, TVN_SELCHANGEDW) => {
                        let event = &*(l.0 as *const NMTREEVIEWW);
                        if let Some(id) = signals
                            .queues
                            .borrow()
                            .get(event.itemNew.lParam.0 as usize)
                            .copied()
                        {
                            signals.push(UiAction::Select(id));
                        }
                    }
                    (SCHED_TABS, TCN_SELCHANGE) => {
                        let page = send(header.hwndFrom, TCM_GETCURSEL, 0, 0).0;
                        signals.push(UiAction::Tab(if page == 0 {
                            Tab::Schedule
                        } else {
                            Tab::Files
                        }));
                    }
                    (_, DTN_DATETIMECHANGE) => signals.push(UiAction::Changed),
                    _ => {}
                }
            }
            WM_CLOSE => {
                signals.push(UiAction::Button(SCHED_CLOSE, None));
                return 1;
            }
            WM_DPICHANGED => signals.push(UiAction::Dpi((w.0 & 0xffff) as u32)),
            WM_SETTINGCHANGE | crate::shell_icons::ASSOCIATIONS_CHANGED => {
                signals.push(UiAction::AssociationsChanged);
            }
            WM_NCDESTROY => {
                signals.hwnd.set(HWND::default());
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            }
            _ => {}
        }
        0
    }
}
