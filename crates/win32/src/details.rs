use crate::{
    bridge,
    commands::*,
    platform::{self, item, resource, send, set_text, wide},
    progress,
    transfer_view::{format_transfer, AutoOpenTracker},
    AppError,
};
use rusty_engine::{Command, JobId, JobSnapshot, JobState};
use std::{cell::Cell, collections::BTreeMap};
use windows::{
    core::PWSTR,
    Win32::{
        Foundation::{COLORREF, HWND, LPARAM, RECT, WPARAM},
        Graphics::Gdi::{
            GetSysColor, GetSysColorBrush, SetBkMode, SetTextColor, COLOR_WINDOW, COLOR_WINDOWTEXT,
            HDC, TRANSPARENT,
        },
        UI::{Controls::*, Input::KeyboardAndMouse::EnableWindow, WindowsAndMessaging::*},
    },
};

const STATUS_FIELDS: [i32; 13] = [
    STATUS_URL,
    STATUS_STATE,
    STATUS_SIZE,
    STATUS_RECEIVED,
    STATUS_RATE,
    STATUS_ETA,
    STATUS_RESUME,
    STATUS_LABEL,
    SIZE_LABEL,
    RECEIVED_LABEL,
    RATE_LABEL,
    ETA_LABEL,
    RESUME_LABEL,
];
struct Detail {
    id: JobId,
    owner: HWND,
    hwnd: Cell<HWND>,
    stop: Cell<bool>,
    pause: Cell<Option<bool>>,
    limit: Cell<Option<Option<u64>>>,
    state: Cell<JobState>,
    limit_initialized: Cell<bool>,
    cancellable: Cell<bool>,
    expanded: Cell<bool>,
    close_on_completion: Cell<bool>,
    marquee: Cell<bool>,
}
#[derive(Default)]
pub struct DetailsWindows {
    windows: BTreeMap<JobId, Box<Detail>>,
    attempts: AutoOpenTracker,
}
impl DetailsWindows {
    pub fn show_existing(&mut self, owner: HWND, snapshot: &JobSnapshot) -> Result<(), AppError> {
        if let Some(detail) = self.windows.get(&snapshot.id) {
            detail.close_on_completion.set(false);
            if let Ok(checkbox) = item(detail.hwnd.get(), COMPLETION_CLOSE) {
                send(checkbox, BM_SETCHECK, BST_UNCHECKED.0 as usize, 0);
            }
        }
        self.show_or_update(owner, snapshot)
    }
    pub fn observe(&mut self, owner: HWND, snapshot: &JobSnapshot) -> Result<(), AppError> {
        if self.attempts.should_open(snapshot) {
            self.show_or_update(owner, snapshot)?;
        } else {
            self.update(snapshot);
        }
        Ok(())
    }
    pub fn show_or_update(&mut self, owner: HWND, snapshot: &JobSnapshot) -> Result<(), AppError> {
        self.prune();
        if let Some(detail) = self.windows.get(&snapshot.id) {
            unsafe {
                let _ = ShowWindow(detail.hwnd.get(), SW_SHOW);
                let _ = SetForegroundWindow(detail.hwnd.get());
            }
        } else {
            let detail = Box::new(Detail {
                id: snapshot.id,
                owner,
                hwnd: Cell::new(HWND::default()),
                stop: Cell::new(false),
                pause: Cell::new(None),
                limit: Cell::new(None),
                state: Cell::new(snapshot.state),
                limit_initialized: Cell::new(false),
                cancellable: Cell::new(false),
                expanded: Cell::new(false),
                close_on_completion: Cell::new(false),
                marquee: Cell::new(false),
            });
            unsafe {
                CreateDialogParamW(
                    Some(platform::instance()?),
                    resource(DETAILS_DIALOG),
                    Some(owner),
                    Some(detail_proc),
                    LPARAM((&*detail as *const Detail) as isize),
                )?;
            }
            self.windows.insert(snapshot.id, detail);
        }
        self.update(snapshot);
        Ok(())
    }
    pub fn update(&self, snapshot: &JobSnapshot) {
        let Some(detail) = self.windows.get(&snapshot.id) else {
            return;
        };
        let hwnd = detail.hwnd.get();
        if hwnd.is_invalid() {
            return;
        }
        let view = format_transfer(snapshot);
        if platform::text(hwnd) != view.title {
            set_text(hwnd, &view.title);
        }
        for (id, text) in [
            (STATUS_URL, view.url.as_str()),
            (STATUS_STATE, &view.status),
            (STATUS_SIZE, &view.size),
            (STATUS_RECEIVED, &view.downloaded),
            (STATUS_RATE, &view.rate),
            (STATUS_ETA, &view.time_left),
            (
                STATUS_RESUME,
                if snapshot.resume_capable {
                    "Yes"
                } else {
                    "No (restarts on resume)"
                },
            ),
            (DETAILS_TEXT, &view.details),
        ] {
            set_value(hwnd, id, text);
        }
        detail.state.set(snapshot.state);
        set_value(
            hwnd,
            LIMIT_NOTICE,
            &snapshot
                .speed_limit
                .map(|n| {
                    format!(
                        "Current limit: {} KB/sec. Changes take effect when you Apply.",
                        n / 1024
                    )
                })
                .unwrap_or_else(|| {
                    "Current limit: Unlimited. Uncheck and Apply to remove a limit.".into()
                }),
        );
        if !detail.limit_initialized.replace(true) {
            if let Ok(checkbox) = item(hwnd, LIMIT_ENABLED) {
                send(
                    checkbox,
                    BM_SETCHECK,
                    if snapshot.speed_limit.is_some() {
                        BST_CHECKED.0 as usize
                    } else {
                        BST_UNCHECKED.0 as usize
                    },
                    0,
                );
            }
            set_value(
                hwnd,
                LIMIT_VALUE,
                &snapshot
                    .speed_limit
                    .map(|n| n / 1024)
                    .unwrap_or(1024)
                    .to_string(),
            );
        }
        let active = snapshot.state.is_active() && snapshot.state != JobState::Cancelling;
        for id in [LIMIT_ENABLED, LIMIT_APPLY] {
            if let Ok(control) = item(hwnd, id) {
                unsafe {
                    let _ = EnableWindow(control, active);
                }
            }
        }
        enable_limit_value(hwnd, active);
        set_value(
            hwnd,
            DETAIL_PAUSE,
            match snapshot.state {
                JobState::Paused => "&Resume",
                JobState::Pausing => "Pausing...",
                _ => "&Pause",
            },
        );
        if let Ok(button) = item(hwnd, DETAIL_PAUSE) {
            unsafe {
                let _ = EnableWindow(
                    button,
                    matches!(snapshot.state, JobState::Running | JobState::Paused),
                );
            }
        }
        detail.cancellable.set(active);
        if let Ok(checkbox) = item(hwnd, COMPLETION_CLOSE) {
            unsafe {
                let _ = EnableWindow(checkbox, !snapshot.state.is_terminal());
            }
        }
        if let Ok(button) = item(hwnd, DETAIL_STOP) {
            set_value(
                hwnd,
                DETAIL_STOP,
                if snapshot.state.is_terminal() {
                    "Close"
                } else {
                    "Cancel"
                },
            );
            unsafe {
                let _ = EnableWindow(button, active || snapshot.state.is_terminal());
            }
        }
        if let Ok(bar) = item(hwnd, PROGRESS) {
            let position = progress::scale(
                snapshot.received,
                snapshot.total,
                snapshot.state == JobState::Completed,
            );
            let marquee = position.is_none() && snapshot.state == JobState::Running;
            if detail.marquee.replace(marquee) != marquee {
                unsafe {
                    let style = GetWindowLongW(bar, GWL_STYLE) as u32;
                    SetWindowLongW(
                        bar,
                        GWL_STYLE,
                        (if marquee {
                            style | PBS_MARQUEE
                        } else {
                            style & !PBS_MARQUEE
                        }) as i32,
                    );
                }
                send(bar, PBM_SETMARQUEE, marquee as usize, 30);
            }
            if !marquee {
                send(bar, PBM_SETRANGE32, 0, 10_000);
                send(bar, PBM_SETPOS, position.unwrap_or(0) as usize, 0);
            }
        }
        if snapshot.state == JobState::Completed && detail.close_on_completion.get() {
            destroy(hwnd);
        }
    }
    pub fn is_dialog_message(&self, msg: &MSG) -> bool {
        self.windows.values().any(|d| {
            let hwnd = d.hwnd.get();
            !hwnd.is_invalid() && unsafe { IsDialogMessageW(hwnd, msg).as_bool() }
        })
    }
    pub fn take_commands(&self) -> Vec<Command> {
        let mut commands = Vec::new();
        for detail in self.windows.values() {
            if detail.stop.replace(false) {
                commands.push(Command::Cancel(detail.id));
            }
            if let Some(pause) = detail.pause.take() {
                commands.push(if pause {
                    Command::Pause(detail.id)
                } else {
                    Command::Resume(detail.id)
                });
            }
            if let Some(limit) = detail.limit.take() {
                commands.push(Command::SetSpeedLimit(detail.id, limit));
            }
        }
        commands
    }
    pub fn remove(&mut self, id: JobId) {
        if let Some(detail) = self.windows.remove(&id) {
            destroy(detail.hwnd.get());
        }
        self.attempts.forget(id);
    }
    pub fn prune(&mut self) {
        self.windows
            .retain(|_, detail| !detail.hwnd.get().is_invalid());
    }
    pub fn close_all(&mut self) {
        for detail in self.windows.values() {
            destroy(detail.hwnd.get());
        }
        self.windows.clear();
    }
}
impl Drop for DetailsWindows {
    fn drop(&mut self) {
        self.close_all();
    }
}
fn set_value(hwnd: HWND, id: i32, value: &str) {
    if let Ok(control) = item(hwnd, id) {
        if platform::text(control) != value {
            set_text(control, value);
        }
    }
}
fn visible(hwnd: HWND, id: i32, show: bool) {
    if let Ok(control) = item(hwnd, id) {
        unsafe {
            let _ = ShowWindow(control, if show { SW_SHOW } else { SW_HIDE });
        }
    }
}
fn select_page(hwnd: HWND) {
    let selected = item(hwnd, STATUS_TABS)
        .map(|tabs| send(tabs, TCM_GETCURSEL, 0, 0).0)
        .unwrap_or(0);
    for id in STATUS_FIELDS {
        visible(hwnd, id, selected == 0);
    }
    for id in [
        LIMIT_NOTICE,
        LIMIT_ENABLED,
        LIMIT_VALUE,
        LIMIT_UNIT,
        LIMIT_APPLY,
    ] {
        visible(hwnd, id, selected == 1);
    }
    visible(hwnd, COMPLETION_CLOSE, selected == 2);
    visible(hwnd, COMPLETION_NOTICE, selected == 2);
}
fn expand(hwnd: HWND, expanded: bool) -> windows::core::Result<()> {
    unsafe {
        let mut target = RECT {
            left: 0,
            top: 0,
            right: 380,
            bottom: if expanded { 228 } else { 145 },
        };
        MapDialogRect(hwnd, &mut target)?;
        let mut outer = RECT::default();
        let mut client = RECT::default();
        GetWindowRect(hwnd, &mut outer)?;
        GetClientRect(hwnd, &mut client)?;
        SetWindowPos(
            hwnd,
            None,
            0,
            0,
            outer.right - outer.left,
            target.bottom + (outer.bottom - outer.top - client.bottom),
            SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
        )?;
    }
    visible(hwnd, DETAILS_TEXT, expanded);
    set_value(
        hwnd,
        DETAIL_TOGGLE,
        if expanded {
            "Hide &details <<"
        } else {
            "Show &details >>"
        },
    );
    Ok(())
}
fn destroy(hwnd: HWND) {
    if !hwnd.is_invalid() {
        unsafe {
            let _ = DestroyWindow(hwnd);
        }
    }
}
fn enable_limit_value(hwnd: HWND, active: bool) {
    let enabled = item(hwnd, LIMIT_ENABLED)
        .is_ok_and(|control| send(control, BM_GETCHECK, 0, 0).0 == BST_CHECKED.0 as isize);
    if let Ok(edit) = item(hwnd, LIMIT_VALUE) {
        unsafe {
            let _ = EnableWindow(edit, active && enabled);
        }
    }
}
fn wake_owner(detail: &Detail) {
    unsafe {
        let _ = PostMessageW(
            Some(detail.owner),
            bridge::ENGINE_WAKE,
            WPARAM(0),
            LPARAM(0),
        );
    }
}
unsafe extern "system" fn detail_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> isize {
    std::panic::catch_unwind(|| unsafe { detail_inner(hwnd, msg, wparam, lparam) }).unwrap_or(0)
}
unsafe fn detail_inner(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> isize {
    unsafe {
        if msg == WM_INITDIALOG {
            if lparam.0 == 0 {
                return 0;
            }
            let detail = &*(lparam.0 as *const Detail);
            detail.hwnd.set(hwnd);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, lparam.0);
            visible(hwnd, DETAILS_TEXT, false);
            if let Ok(tabs) = item(hwnd, STATUS_TABS) {
                for (index, title) in ["Download status", "Speed Limiter", "Options on completion"]
                    .iter()
                    .enumerate()
                {
                    let mut text = wide(title);
                    let tab = TCITEMW {
                        mask: TCIF_TEXT,
                        pszText: PWSTR(text.as_mut_ptr()),
                        ..Default::default()
                    };
                    send(tabs, TCM_INSERTITEMW, index, &tab as *const _ as isize);
                }
            }
            select_page(hwnd);
            if let Ok(icon) = LoadIconW(None, IDI_APPLICATION) {
                send(hwnd, WM_SETICON, ICON_SMALL as usize, icon.0 as isize);
            }
            let _ = ShowWindow(hwnd, SW_SHOW);
            return 1;
        }
        let pointer = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Detail;
        if pointer.is_null() {
            return 0;
        }
        let detail = &*pointer;
        match msg {
            WM_NOTIFY if lparam.0 != 0 => {
                let header = &*(lparam.0 as *const NMHDR);
                if header.idFrom == STATUS_TABS as usize && header.code == TCN_SELCHANGE {
                    select_page(hwnd);
                    return 1;
                }
            }
            WM_COMMAND => match (wparam.0 & 0xffff) as i32 {
                DETAIL_PAUSE => {
                    match detail.state.get() {
                        JobState::Running => detail.pause.set(Some(true)),
                        JobState::Paused => detail.pause.set(Some(false)),
                        _ => return 1,
                    }
                    wake_owner(detail);
                    return 1;
                }
                LIMIT_ENABLED => {
                    enable_limit_value(hwnd, detail.cancellable.get());
                    return 1;
                }
                LIMIT_APPLY => {
                    if !detail.cancellable.get() {
                        return 1;
                    }
                    let enabled = item(hwnd, LIMIT_ENABLED).is_ok_and(|control| {
                        send(control, BM_GETCHECK, 0, 0).0 == BST_CHECKED.0 as isize
                    });
                    let limit = if enabled {
                        let value = item(hwnd, LIMIT_VALUE)
                            .ok()
                            .map(platform::text)
                            .unwrap_or_default();
                        match value
                            .trim()
                            .parse::<u64>()
                            .ok()
                            .filter(|n| *n > 0)
                            .and_then(|n| n.checked_mul(1024))
                        {
                            Some(limit) => Some(limit),
                            None => {
                                platform::message(
                                    Some(hwnd),
                                    "Speed Limiter",
                                    "Enter a positive whole number of KB/sec.",
                                    false,
                                );
                                return 1;
                            }
                        }
                    } else {
                        None
                    };
                    detail.limit.set(Some(limit));
                    wake_owner(detail);
                    return 1;
                }
                DETAIL_STOP => {
                    if detail.cancellable.get() {
                        detail.stop.set(true);
                        if let Ok(button) = item(hwnd, DETAIL_STOP) {
                            let _ = EnableWindow(button, false);
                        }
                        let _ = PostMessageW(
                            Some(detail.owner),
                            bridge::ENGINE_WAKE,
                            WPARAM(0),
                            LPARAM(0),
                        );
                    } else {
                        destroy(hwnd);
                    }
                    return 1;
                }
                DETAIL_TOGGLE => {
                    let expanded = !detail.expanded.get();
                    match expand(hwnd, expanded) {
                        Ok(()) => detail.expanded.set(expanded),
                        Err(_) => {
                            platform::message(
                                Some(hwnd),
                                "Download status",
                                "The details panel could not be resized.",
                                false,
                            );
                        }
                    }
                    return 1;
                }
                COMPLETION_CLOSE => {
                    if let Ok(control) = item(hwnd, COMPLETION_CLOSE) {
                        detail
                            .close_on_completion
                            .set(send(control, BM_GETCHECK, 0, 0).0 == BST_CHECKED.0 as isize);
                    }
                    return 1;
                }
                2 => {
                    destroy(hwnd);
                    return 1;
                }
                _ => {}
            },
            WM_CLOSE => {
                destroy(hwnd);
                return 1;
            }
            WM_CTLCOLORSTATIC | WM_CTLCOLORBTN if lparam.0 != 0 => {
                let control_id = GetDlgCtrlID(HWND(lparam.0 as *mut _));
                if STATUS_FIELDS.contains(&control_id)
                    || [
                        LIMIT_NOTICE,
                        LIMIT_ENABLED,
                        LIMIT_UNIT,
                        COMPLETION_CLOSE,
                        COMPLETION_NOTICE,
                    ]
                    .contains(&control_id)
                {
                    let dc = HDC(wparam.0 as *mut _);
                    SetTextColor(
                        dc,
                        if (STATUS_STATE..=STATUS_RESUME).contains(&control_id) {
                            COLORREF(0x00FF0000)
                        } else {
                            COLORREF(GetSysColor(COLOR_WINDOWTEXT))
                        },
                    );
                    SetBkMode(dc, TRANSPARENT);
                    return GetSysColorBrush(COLOR_WINDOW).0 as isize;
                }
            }
            WM_NCDESTROY => {
                detail.hwnd.set(HWND::default());
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            }
            _ => {}
        }
        0
    }
}
