//! Modeless native project manager; HWND access stays on the UI thread.
use crate::{
    commands::*,
    platform::{self, item, send, set_text, text, wide},
    AppError,
};
use rusty_engine::{grabber::*, Command, EngineClient};
use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    path::PathBuf,
    sync::mpsc,
    time::{Duration, Instant},
};
use windows::{
    core::{PCWSTR, PWSTR},
    Win32::{
        Foundation::{HWND, LPARAM, WPARAM},
        UI::{Controls::*, HiDpi::GetDpiForWindow, Shell::ShellExecuteW, WindowsAndMessaging::*},
    },
};

enum Action {
    Button(i32),
    Project,
    Toggle(ItemId, bool),
    Close,
    Dpi,
}
#[derive(Default)]
struct Signals {
    hwnd: Cell<HWND>,
    suppress: Cell<bool>,
    actions: RefCell<VecDeque<Action>>,
    rows: RefCell<Vec<ItemId>>,
}
impl Signals {
    fn push(&self, action: Action) {
        if !self.suppress.get() {
            let mut q = self.actions.borrow_mut();
            if q.len() < 512 {
                q.push_back(action);
            }
        }
    }
}
struct Pending {
    receive: mpsc::Receiver<Result<GrabberReply, String>>,
    load: bool,
    view: bool,
    started: Instant,
}
struct Manager {
    signals: Box<Signals>,
    engine: EngineClient,
    projects: Vec<ProjectId>,
    project_names: Vec<String>,
    selected: Option<ProjectId>,
    snapshot: Option<ProjectSnapshot>,
    rows: Vec<GrabberItem>,
    offset: usize,
    total: usize,
    filter: String,
    pending: Option<Pending>,
    queued: VecDeque<(GrabberAction, bool)>,
    last_query: Instant,
    default_destination: PathBuf,
    default_options: rusty_engine::DownloadOptions,
}
#[derive(Default)]
pub struct GrabberWindow {
    manager: Option<Manager>,
    error: Option<String>,
}
impl GrabberWindow {
    pub fn observe(&mut self, event: &GrabberEvent) {
        self.error = event.error.clone();
        if let (Some(manager), Some(error)) = (&self.manager, &self.error) {
            manager.notice(error);
        }
    }
    pub fn open(
        &mut self,
        owner: HWND,
        engine: EngineClient,
        destination: PathBuf,
        options: rusty_engine::DownloadOptions,
    ) -> Result<(), AppError> {
        if let Some(manager) = &self.manager {
            if !manager.signals.hwnd.get().is_invalid() {
                platform::bring_to_front(manager.signals.hwnd.get());
                return Ok(());
            }
        }
        self.close_all();
        let signals = Box::<Signals>::default();
        let hwnd = unsafe {
            CreateDialogParamW(
                Some(platform::instance()?),
                platform::resource(GRABBER_DIALOG),
                Some(owner),
                Some(callback),
                LPARAM((&*signals as *const Signals) as isize),
            )
        }?;
        signals.hwnd.set(hwnd);
        let mut manager = Manager {
            signals,
            engine,
            projects: vec![],
            project_names: vec![],
            selected: None,
            snapshot: None,
            rows: vec![],
            offset: 0,
            total: 0,
            filter: String::new(),
            pending: None,
            queued: VecDeque::new(),
            last_query: Instant::now(),
            default_destination: destination.clone(),
            default_options: options.clone(),
        };
        manager.signals.suppress.set(true);
        let mode = item(hwnd, GRAB_MODE)?;
        for label in ["Collect files", "Offline website"] {
            let label = wide(label);
            send(mode, CB_ADDSTRING, 0, label.as_ptr() as isize);
        }
        send(mode, CB_SETCURSEL, 0, 0);
        let list = item(hwnd, GRAB_LIST)?;
        send(
            list,
            LVM_SETEXTENDEDLISTVIEWSTYLE,
            0,
            (LVS_EX_FULLROWSELECT | LVS_EX_CHECKBOXES | LVS_EX_DOUBLEBUFFER) as isize,
        );
        for (column, (label, width)) in [
            ("URL", 470),
            ("Type", 130),
            ("Size", 85),
            ("Depth", 50),
            ("Status", 220),
        ]
        .into_iter()
        .enumerate()
        {
            let mut label = wide(label);
            let mut col = LVCOLUMNW {
                mask: LVCF_TEXT | LVCF_WIDTH,
                cx: width,
                pszText: PWSTR(label.as_mut_ptr()),
                ..Default::default()
            };
            send(list, LVM_INSERTCOLUMNW, column, &mut col as *mut _ as isize);
        }
        manager.columns();
        manager.write(&ProjectConfig {
            destination,
            transfer_options: options,
            ..Default::default()
        });
        manager.signals.suppress.set(false);
        manager.enqueue(
            GrabberAction::Query {
                project: None,
                offset: 0,
                filter: String::new(),
            },
            true,
        );
        platform::bring_to_front(hwnd);
        self.manager = Some(manager);
        Ok(())
    }
    pub fn is_dialog_message(&self, msg: &MSG) -> bool {
        self.manager.as_ref().is_some_and(|m| {
            !m.signals.hwnd.get().is_invalid()
                && unsafe { IsDialogMessageW(m.signals.hwnd.get(), msg).as_bool() }
        })
    }
    pub fn close_all(&mut self) {
        if let Some(manager) = self.manager.take() {
            drop(manager);
        }
    }
    pub fn poll(&mut self) -> Result<(), AppError> {
        let Some(manager) = self.manager.as_mut() else {
            return Ok(());
        };
        if manager.signals.hwnd.get().is_invalid() {
            self.close_all();
            return Ok(());
        }
        loop {
            let action = { manager.signals.actions.borrow_mut().pop_front() };
            let Some(action) = action else {
                break;
            };
            match action {
                Action::Close => {
                    self.close_all();
                    return Ok(());
                }
                Action::Dpi => manager.columns(),
                Action::Project => {
                    let index = send(item(manager.hwnd(), GRAB_PROJECT)?, CB_GETCURSEL, 0, 0).0;
                    if manager.pending.is_some() || !manager.queued.is_empty() {
                        manager.notice(
                            "Wait for the pending project command before switching projects.",
                        );
                        continue;
                    }
                    manager.selected = manager.projects.get(index as usize).cloned();
                    manager.offset = 0;
                    manager.filter.clear();
                    manager.rows.clear();
                    manager.enqueue(
                        GrabberAction::Query {
                            project: manager.selected.clone(),
                            offset: 0,
                            filter: String::new(),
                        },
                        true,
                    );
                }
                Action::Toggle(id, selected) => {
                    if let Some(project) = manager.selected.clone() {
                        manager.enqueue(
                            GrabberAction::Select {
                                project,
                                items: vec![id],
                                selected,
                            },
                            false,
                        );
                    }
                }
                Action::Button(id) => {
                    if let Err(error) = manager.button(id) {
                        manager.notice(&error);
                    }
                }
            }
        }
        let response = manager
            .pending
            .as_ref()
            .and_then(|p| match p.receive.try_recv() {
                Ok(r) => Some((p.load, p.view, r)),
                Err(mpsc::TryRecvError::Disconnected) => Some((
                    p.load,
                    p.view,
                    Err("Site Grabber command was interrupted.".into()),
                )),
                Err(_) => None,
            });
        if let Some((load, view, result)) = response {
            manager.pending = None;
            match result {
                Ok(mut reply) => {
                    if !view {
                        reply.results = manager.rows.clone();
                        reply.offset = manager.offset;
                        reply.total = manager.total;
                    }
                    manager.draw(reply, load)?;
                }
                Err(error) => manager.notice(&error),
            }
            manager.last_query = Instant::now();
        }
        if manager
            .pending
            .as_ref()
            .is_some_and(|p| p.started.elapsed() > Duration::from_secs(35))
        {
            manager.pending = None;
            manager.notice("Project command acknowledgement timed out. Refresh or Resume to reconcile saved progress.");
        }
        if manager.pending.is_none() {
            if let Some((action, load)) = manager.queued.pop_front() {
                manager.dispatch(action, load);
            } else if manager.last_query.elapsed() >= Duration::from_secs(1) {
                manager.dispatch(
                    GrabberAction::Query {
                        project: manager.selected.clone(),
                        offset: manager.offset,
                        filter: manager.filter.clone(),
                    },
                    false,
                );
            }
        }
        Ok(())
    }
}
impl Manager {
    fn hwnd(&self) -> HWND {
        self.signals.hwnd.get()
    }
    fn notice(&self, value: &str) {
        if let Ok(control) = item(self.hwnd(), GRAB_STATUS) {
            set_text(control, value);
        }
    }
    fn enqueue(&mut self, action: GrabberAction, load: bool) {
        if self.queued.len() < 256 {
            self.queued.push_back((action, load));
        } else {
            self.notice("Too many pending changes. Wait for Site Grabber to catch up.");
        }
    }
    fn dispatch(&mut self, action: GrabberAction, load: bool) {
        let view = load || matches!(&action, GrabberAction::Query { .. });
        let (reply, receive) = mpsc::sync_channel(1);
        match self
            .engine
            .try_send(Command::Grabber(GrabberCommand { action, reply }))
        {
            Ok(()) => {
                self.pending = Some(Pending {
                    receive,
                    load,
                    view,
                    started: Instant::now(),
                })
            }
            Err(error) => self.notice(&error.to_string()),
        }
    }
    fn value(&self, id: i32) -> String {
        item(self.hwnd(), id).map(text).unwrap_or_default()
    }
    fn write(&self, c: &ProjectConfig) {
        for (id, value) in [
            (GRAB_NAME, c.name.clone()),
            (GRAB_URL, c.start_url.clone()),
            (GRAB_DESTINATION, c.destination.to_string_lossy().into()),
            (GRAB_PAGE_HOSTS, c.page_hosts.join(", ")),
            (GRAB_ASSET_HOSTS, c.asset_hosts.join(", ")),
            (GRAB_INCLUDE, c.include_urls.join(", ")),
            (GRAB_EXCLUDE, c.exclude_urls.join(", ")),
            (GRAB_EXTENSIONS, c.extensions.join(", ")),
            (GRAB_MIME, c.mime_types.join(", ")),
            (GRAB_DEPTH, c.max_depth.to_string()),
            (GRAB_PAGES, c.max_pages.to_string()),
            (GRAB_RESULTS, c.max_results.to_string()),
            (GRAB_WORKERS, c.scan_workers.to_string()),
            (GRAB_DELAY, c.host_delay_ms.to_string()),
            (GRAB_REDIRECTS, c.max_redirects.to_string()),
            (GRAB_DOCUMENT, (c.document_limit / 1024 / 1024).to_string()),
            (GRAB_CACHE, (c.cache_limit / 1024 / 1024).to_string()),
        ] {
            if let Ok(control) = item(self.hwnd(), id) {
                set_text(control, &value);
            }
        }
        if let Ok(mode) = item(self.hwnd(), GRAB_MODE) {
            send(
                mode,
                CB_SETCURSEL,
                usize::from(c.mode == GrabberMode::OfflineWebsite),
                0,
            );
        }
        if let Ok(robots) = item(self.hwnd(), GRAB_ROBOTS) {
            send(
                robots,
                BM_SETCHECK,
                if c.respect_robots {
                    BST_CHECKED.0 as usize
                } else {
                    0
                },
                0,
            );
        }
    }
    fn read(&self) -> Result<ProjectConfig, String> {
        fn list(value: String) -> Vec<String> {
            value
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
                .collect()
        }
        let number = |id, name: &str| {
            self.value(id)
                .trim()
                .parse::<u64>()
                .map_err(|_| format!("Enter a valid {name}."))
        };
        let mut config = ProjectConfig {
            transfer_options: self.default_options.clone(),
            name: self.value(GRAB_NAME),
            start_url: self.value(GRAB_URL),
            destination: PathBuf::from(self.value(GRAB_DESTINATION)),
            mode: if item(self.hwnd(), GRAB_MODE).is_ok_and(|h| send(h, CB_GETCURSEL, 0, 0).0 == 1)
            {
                GrabberMode::OfflineWebsite
            } else {
                GrabberMode::CollectFiles
            },
            page_hosts: list(self.value(GRAB_PAGE_HOSTS)),
            asset_hosts: list(self.value(GRAB_ASSET_HOSTS)),
            include_urls: list(self.value(GRAB_INCLUDE)),
            exclude_urls: list(self.value(GRAB_EXCLUDE)),
            extensions: list(self.value(GRAB_EXTENSIONS)),
            mime_types: list(self.value(GRAB_MIME)),
            max_depth: number(GRAB_DEPTH, "depth")?
                .try_into()
                .map_err(|_| "Depth is too large.")?,
            max_pages: number(GRAB_PAGES, "page limit")?
                .try_into()
                .map_err(|_| "Page limit is too large.")?,
            max_results: number(GRAB_RESULTS, "result limit")?
                .try_into()
                .map_err(|_| "Result limit is too large.")?,
            scan_workers: number(GRAB_WORKERS, "worker count")?
                .try_into()
                .map_err(|_| "Worker count is too large.")?,
            host_delay_ms: number(GRAB_DELAY, "host delay")?,
            max_redirects: number(GRAB_REDIRECTS, "redirect limit")?
                .try_into()
                .map_err(|_| "Redirect limit is too large.")?,
            document_limit: number(GRAB_DOCUMENT, "document limit in MiB")?
                .checked_mul(1024 * 1024)
                .ok_or("Document limit is too large.")?,
            cache_limit: number(GRAB_CACHE, "cache limit in MiB")?
                .checked_mul(1024 * 1024)
                .ok_or("Cache limit is too large.")?,
            respect_robots: item(self.hwnd(), GRAB_ROBOTS)
                .is_ok_and(|h| send(h, BM_GETCHECK, 0, 0).0 == BST_CHECKED.0 as isize),
        };
        config.validate()?;
        Ok(config)
    }
    fn button(&mut self, id: i32) -> Result<(), String> {
        if id == GRAB_CLOSE {
            self.signals.push(Action::Close);
            return Ok(());
        }
        if id == GRAB_NEW {
            if self.pending.is_some() || !self.queued.is_empty() {
                return Err("Wait for the pending command before creating a project.".into());
            }
            self.selected = None;
            self.snapshot = None;
            self.offset = 0;
            self.rows.clear();
            self.signals.suppress.set(true);
            if let Ok(combo) = item(self.hwnd(), GRAB_PROJECT) {
                send(combo, CB_SETCURSEL, usize::MAX, 0);
            }
            if let Ok(list) = item(self.hwnd(), GRAB_LIST) {
                send(list, LVM_DELETEALLITEMS, 0, 0);
            }
            self.signals.rows.borrow_mut().clear();
            self.write(&ProjectConfig {
                destination: self.default_destination.clone(),
                transfer_options: self.default_options.clone(),
                ..Default::default()
            });
            self.signals.suppress.set(false);
            self.notice("Enter project settings, then Save settings.");
            return Ok(());
        }
        if matches!(id, GRAB_SAVE | GRAB_RESCAN) {
            if !self.queued.is_empty() || self.pending.as_ref().is_some_and(|p| p.load) {
                return Err("Wait for the pending project command before saving again.".into());
            }
            let config = self.read()?;
            if id==GRAB_RESCAN&&self.selected.is_some()&&!platform::message(Some(self.hwnd()),"Rescan website","Stop old downloads and start a new scan generation? Existing exported snapshots will be preserved.",true){return Ok(());}
            self.enqueue(
                GrabberAction::Save {
                    project: self.selected.clone(),
                    config,
                    rescan: id == GRAB_RESCAN,
                },
                true,
            );
            return Ok(());
        }
        if matches!(id, GRAB_FIND | GRAB_PREVIOUS | GRAB_NEXT) {
            if id == GRAB_FIND {
                self.filter = self.value(GRAB_FILTER);
                self.offset = 0;
            }
            if id == GRAB_PREVIOUS {
                self.offset = self.offset.saturating_sub(200);
            }
            if id == GRAB_NEXT && self.offset + 200 < self.total {
                self.offset += 200;
            }
            self.enqueue(
                GrabberAction::Query {
                    project: self.selected.clone(),
                    offset: self.offset,
                    filter: self.filter.clone(),
                },
                false,
            );
            return Ok(());
        }
        let project = self
            .selected
            .clone()
            .ok_or("Save or select a project first.")?;
        match id{
            GRAB_SCAN=>self.enqueue(GrabberAction::Scan(project),false),GRAB_STOP=>self.enqueue(GrabberAction::Stop(project),false),
            GRAB_RESUME=>self.enqueue(GrabberAction::Resume(project),false),GRAB_DOWNLOAD=>self.enqueue(GrabberAction::Download(project),false),
            GRAB_RETRY=>self.enqueue(GrabberAction::RetryFailed(project),false),
            GRAB_DELETE=>{if platform::message(Some(self.hwnd()),"Delete grabber project","Stop owned downloads and delete this project's metadata and cache? Exported files and ordinary download history will be preserved.",true){self.enqueue(GrabberAction::Delete(project),false);}},
            GRAB_SELECT_PAGE|GRAB_CLEAR_PAGE=>self.enqueue(GrabberAction::Select{project,items:self.rows.iter().map(|i|i.id).collect(),selected:id==GRAB_SELECT_PAGE},false),
            GRAB_FOLDER=>{let path=self.snapshot.as_ref().map(|s|s.destination.clone()).ok_or("Project information is unavailable.")?;self.open_path(path,false)?;},
            GRAB_OFFLINE=>{let path=self.snapshot.as_ref().and_then(|s|s.offline.clone()).ok_or("No published offline snapshot is available.")?;self.open_path(path,true)?;},_=>{}
        }
        Ok(())
    }
    fn open_path(&self, path: PathBuf, offline: bool) -> Result<(), String> {
        if offline && (!path.is_file() || path.file_name().is_none_or(|v| v != "index.html")) {
            return Err("The offline entry point is missing.".into());
        }
        if !offline && !path.is_dir() {
            return Err("The destination folder does not exist yet.".into());
        }
        let path = wide(path);
        let open = wide("open");
        let result = unsafe {
            ShellExecuteW(
                Some(self.hwnd()),
                PCWSTR(open.as_ptr()),
                PCWSTR(path.as_ptr()),
                None,
                None,
                SW_SHOWNORMAL,
            )
        };
        if result.0 as isize <= 32 {
            return Err("Windows could not open this project path.".into());
        }
        Ok(())
    }
    fn columns(&self) {
        if let Ok(list) = item(self.hwnd(), GRAB_LIST) {
            let dpi = unsafe { GetDpiForWindow(self.hwnd()) }.max(96);
            for (i, width) in [470, 130, 85, 50, 220].into_iter().enumerate() {
                send(list, LVM_SETCOLUMNWIDTH, i, (width * dpi / 96) as isize);
            }
        }
    }
    fn draw(&mut self, reply: GrabberReply, load: bool) -> Result<(), AppError> {
        self.signals.suppress.set(true);
        let result = self.draw_inner(reply, load);
        self.signals.suppress.set(false);
        result
    }
    fn draw_inner(&mut self, reply: GrabberReply, load: bool) -> Result<(), AppError> {
        if load {
            if let Some(project) = &reply.project {
                self.selected = Some(project.id.clone());
                self.offset = reply.offset;
            }
            if let Some(config) = &reply.config {
                self.write(config);
            }
        }
        if self
            .selected
            .as_ref()
            .is_some_and(|id| !reply.projects.iter().any(|p| &p.id == id))
        {
            self.selected = None;
            self.snapshot = None;
        }
        let ids: Vec<_> = reply.projects.iter().map(|p| p.id.clone()).collect();
        let names: Vec<_> = reply.projects.iter().map(|p| p.name.clone()).collect();
        if ids != self.projects || names != self.project_names {
            let combo = item(self.hwnd(), GRAB_PROJECT)?;
            send(combo, CB_RESETCONTENT, 0, 0);
            for p in &reply.projects {
                let name = wide(&p.name);
                send(combo, CB_ADDSTRING, 0, name.as_ptr() as isize);
            }
            self.projects = ids;
            self.project_names = names;
        }
        let selected = self
            .selected
            .as_ref()
            .and_then(|id| self.projects.iter().position(|p| p == id));
        send(
            item(self.hwnd(), GRAB_PROJECT)?,
            CB_SETCURSEL,
            selected.unwrap_or(usize::MAX),
            0,
        );
        self.total = reply.total;
        self.offset = reply.offset;
        if let Some(project) = reply.project {
            if self.selected.as_ref() == Some(&project.id) {
                self.notice(&format!(
                    "{:?} — {} results, {} selected, {} complete. Rows {}–{} of {}. {}",
                    project.state,
                    project.results,
                    project.selected,
                    project.completed,
                    if reply.total == 0 {
                        0
                    } else {
                        reply.offset + 1
                    },
                    (reply.offset + reply.results.len()).min(reply.total),
                    reply.total,
                    project.message
                ));
                self.snapshot = Some(project);
            }
        }
        if self.rows != reply.results {
            let list = item(self.hwnd(), GRAB_LIST)?;
            send(list, WM_SETREDRAW, 0, 0);
            send(list, LVM_DELETEALLITEMS, 0, 0);
            self.signals
                .rows
                .replace(reply.results.iter().map(|i| i.id).collect());
            for (row, item) in reply.results.iter().enumerate() {
                let mut url = wide(&item.url);
                let mut lv = LVITEMW {
                    mask: LVIF_TEXT | LVIF_STATE,
                    iItem: row as i32,
                    pszText: PWSTR(url.as_mut_ptr()),
                    stateMask: LVIS_STATEIMAGEMASK,
                    state: LIST_VIEW_ITEM_STATE_FLAGS(if item.selected {
                        2 << 12
                    } else {
                        1 << 12
                    }),
                    ..Default::default()
                };
                send(list, LVM_INSERTITEMW, 0, &mut lv as *mut _ as isize);
                let kind = if item.mime.is_empty() {
                    format!("{:?}", item.kind)
                } else {
                    item.mime.clone()
                };
                for (column, value) in [
                    kind,
                    item.size
                        .map(|n| format!("{n} B"))
                        .unwrap_or_else(|| "Unknown".into()),
                    item.depth.to_string(),
                    format!("{:?} {}", item.status, item.message),
                ]
                .into_iter()
                .enumerate()
                {
                    let mut value = wide(value);
                    let mut sub = LVITEMW {
                        iSubItem: column as i32 + 1,
                        pszText: PWSTR(value.as_mut_ptr()),
                        ..Default::default()
                    };
                    send(list, LVM_SETITEMTEXTW, row, &mut sub as *mut _ as isize);
                }
            }
            self.rows = reply.results;
            send(list, WM_SETREDRAW, 1, 0);
            unsafe {
                let _ = windows::Win32::Graphics::Gdi::InvalidateRect(Some(list), None, true);
            }
        }
        Ok(())
    }
}
impl Drop for Manager {
    fn drop(&mut self) {
        let hwnd = self.signals.hwnd.get();
        if !hwnd.is_invalid() {
            unsafe {
                let _ = DestroyWindow(hwnd);
            }
        }
    }
}
unsafe extern "system" fn callback(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> isize {
    std::panic::catch_unwind(|| unsafe { inner(hwnd, msg, w, l) }).unwrap_or(0)
}
unsafe fn inner(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> isize {
    unsafe {
        if msg == WM_INITDIALOG {
            if l.0 != 0 {
                let signals = &*(l.0 as *const Signals);
                signals.hwnd.set(hwnd);
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, l.0);
                return 1;
            }
            return 0;
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
                if id == 2 || id == GRAB_CLOSE {
                    signals.push(Action::Close);
                    return 1;
                }
                if id == GRAB_PROJECT && notification == CBN_SELCHANGE {
                    signals.push(Action::Project);
                    return 1;
                }
                if notification == BN_CLICKED {
                    signals.push(Action::Button(id));
                    return 1;
                }
            }
            WM_NOTIFY if l.0 != 0 => {
                let header = &*(l.0 as *const NMHDR);
                if header.idFrom == GRAB_LIST as usize
                    && header.code == LVN_ITEMCHANGED
                    && !signals.suppress.get()
                {
                    let change = &*(l.0 as *const NMLISTVIEW);
                    if change.iItem >= 0
                        && (change.uOldState ^ change.uNewState) & LVIS_STATEIMAGEMASK.0 != 0
                    {
                        let id = { signals.rows.borrow().get(change.iItem as usize).copied() };
                        if let Some(id) = id {
                            signals.push(Action::Toggle(id, (change.uNewState >> 12) == 2));
                        }
                    }
                }
            }
            WM_CLOSE => {
                signals.push(Action::Close);
                return 1;
            }
            WM_DPICHANGED => signals.push(Action::Dpi),
            WM_NCDESTROY => {
                signals.hwnd.set(HWND::default());
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            }
            _ => {}
        }
        0
    }
}
