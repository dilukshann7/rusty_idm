//! Small native modal loop with stable, signal-only dialog callbacks.
use crate::{
    platform::{self, item, resource, send, set_text, text, wide},
    AppError,
};
use std::cell::{Cell, RefCell};
use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::{HWND, LPARAM, WPARAM},
        UI::{
            Controls::*,
            Input::KeyboardAndMouse::{EnableWindow, IsWindowEnabled},
            WindowsAndMessaging::*,
        },
    },
};
#[derive(Clone, Copy)]
pub enum Action {
    Command(HWND, i32, u32),
    Tab,
    Timer,
    Dpi,
}
#[derive(Default)]
struct Signals {
    hwnd: Cell<HWND>,
    actions: RefCell<Vec<Action>>,
}
impl Signals {
    fn push(&self, action: Action) {
        let mut q = self.actions.borrow_mut();
        if q.len() < 128 {
            q.push(action);
        }
    }
}
pub struct Form {
    signals: Box<Signals>,
    owner: HWND,
    restore: bool,
}
impl Form {
    pub fn new(owner: HWND, id: u16, title: &str) -> Result<Self, AppError> {
        let signals = Box::<Signals>::default();
        let hwnd = unsafe {
            CreateDialogParamW(
                Some(platform::instance()?),
                resource(id),
                Some(owner),
                Some(dialog),
                LPARAM((&*signals as *const Signals) as isize),
            )
        }?;
        signals.hwnd.set(hwnd);
        let form = Self {
            signals,
            owner,
            restore: false,
        };
        let title = wide(title);
        unsafe {
            SetWindowTextW(hwnd, PCWSTR(title.as_ptr()))?;
        }
        Ok(form)
    }
    pub fn hwnd(&self) -> HWND {
        self.signals.hwnd.get()
    }
    pub fn child(&self, id: u16) -> Result<HWND, AppError> {
        Ok(unsafe {
            CreateDialogParamW(
                Some(platform::instance()?),
                resource(id),
                Some(self.hwnd()),
                Some(child_dialog),
                LPARAM((&*self.signals as *const Signals) as isize),
            )
        }?)
    }
    pub fn show(&mut self) {
        self.restore = unsafe { IsWindowEnabled(self.owner) }.as_bool();
        if self.restore {
            unsafe {
                let _ = EnableWindow(self.owner, false);
            }
        }
        platform::bring_to_front(self.hwnd());
    }
    pub fn next(&self) -> Result<Option<Vec<Action>>, AppError> {
        let mut msg = MSG::default();
        let code = unsafe { GetMessageW(&mut msg, None, 0, 0) }.0;
        if code < 0 {
            return Err(AppError("The configuration message loop failed.".into()));
        }
        if code == 0 {
            unsafe {
                PostQuitMessage(msg.wParam.0 as i32);
            }
            return Ok(None);
        }
        if !unsafe { IsDialogMessageW(self.hwnd(), &msg) }.as_bool() {
            unsafe {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
        Ok(Some(self.signals.actions.borrow_mut().drain(..).collect()))
    }
}
impl Drop for Form {
    fn drop(&mut self) {
        unsafe {
            if !self.hwnd().is_invalid() {
                let _ = DestroyWindow(self.hwnd());
            }
            if self.restore {
                let _ = EnableWindow(self.owner, true);
                let _ = SetActiveWindow(self.owner);
            }
        }
    }
}
pub fn value(hwnd: HWND, id: i32) -> String {
    item(hwnd, id).map(text).unwrap_or_default()
}
pub fn put(hwnd: HWND, id: i32, value: &str) {
    if let Ok(h) = item(hwnd, id) {
        if text(h) != value {
            set_text(h, value);
        }
    }
}
pub fn check(hwnd: HWND, id: i32, on: bool) {
    if let Ok(h) = item(hwnd, id) {
        let state = if on { BST_CHECKED.0 } else { BST_UNCHECKED.0 };
        if send(h, BM_GETCHECK, 0, 0).0 != state as isize {
            send(h, BM_SETCHECK, state as usize, 0);
        }
    }
}
pub fn checked(hwnd: HWND, id: i32) -> bool {
    item(hwnd, id).is_ok_and(|h| send(h, BM_GETCHECK, 0, 0).0 == BST_CHECKED.0 as isize)
}
pub fn enabled(hwnd: HWND, id: i32, on: bool) {
    if let Ok(h) = item(hwnd, id) {
        unsafe {
            if IsWindowEnabled(h).as_bool() != on {
                let _ = EnableWindow(h, on);
            }
        }
    }
}
pub fn choices(hwnd: HWND, id: i32, values: &[&str], selected: usize) {
    if let Ok(h) = item(hwnd, id) {
        send(h, CB_RESETCONTENT, 0, 0);
        for value in values {
            let value = wide(value);
            send(h, CB_ADDSTRING, 0, value.as_ptr() as isize);
        }
        send(h, CB_SETCURSEL, selected, 0);
    }
}
pub fn selection(hwnd: HWND, id: i32) -> Option<usize> {
    let n = item(hwnd, id).ok().map(|h| send(h, CB_GETCURSEL, 0, 0).0)?;
    (n >= 0).then_some(n as usize)
}
pub fn limit(hwnd: HWND, id: i32, n: usize) {
    if let Ok(h) = item(hwnd, id) {
        send(h, EM_SETLIMITTEXT, n, 0);
    }
}
unsafe extern "system" fn dialog(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> isize {
    std::panic::catch_unwind(|| unsafe { inner(hwnd, msg, w, l, true) }).unwrap_or(0)
}
unsafe extern "system" fn child_dialog(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> isize {
    std::panic::catch_unwind(|| unsafe { inner(hwnd, msg, w, l, false) }).unwrap_or(0)
}
unsafe fn inner(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM, root: bool) -> isize {
    unsafe {
        if msg == WM_INITDIALOG {
            if l.0 == 0 {
                return 0;
            }
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, l.0);
            if root {
                (&*(l.0 as *const Signals)).hwnd.set(hwnd);
            }
            return 1;
        }
        let p = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Signals;
        if p.is_null() {
            return 0;
        }
        let signals = &*p;
        match msg {
            WM_COMMAND => {
                let notification = (w.0 >> 16) as u32;
                if notification == BN_CLICKED || notification == CBN_SELCHANGE {
                    signals.push(Action::Command(hwnd, (w.0 & 0xffff) as i32, notification));
                }
                return 1;
            }
            WM_NOTIFY if l.0 != 0 => {
                let header = &*(l.0 as *const NMHDR);
                if header.code == TCN_SELCHANGE {
                    signals.push(Action::Tab);
                }
            }
            WM_TIMER => signals.push(Action::Timer),
            WM_DPICHANGED => signals.push(Action::Dpi),
            WM_CLOSE if root => {
                signals.push(Action::Command(hwnd, 2, 0));
                return 1;
            }
            WM_NCDESTROY => {
                if root {
                    signals.hwnd.set(HWND::default());
                }
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            }
            _ => {}
        }
        0
    }
}
