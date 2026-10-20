use crate::{
    commands::*,
    platform::{self, item, resource, set_text, text, wide},
    AppError,
};
use std::cell::RefCell;
use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::{HWND, LPARAM, WPARAM},
        UI::{Controls::EM_SETLIMITTEXT, WindowsAndMessaging::*},
    },
};
struct Context {
    title: String,
    initial: String,
    credentials: bool,
    result: RefCell<Option<(String, String)>>,
}
pub fn name(owner: HWND, title: &str, initial: &str) -> Result<Option<String>, AppError> {
    Ok(show(owner, title, initial, false)?.map(|(name, _)| name))
}
pub fn credentials(owner: HWND) -> Result<Option<(String, String)>, AppError> {
    show(owner, "Download credentials", "", true)
}
fn show(
    owner: HWND,
    title: &str,
    initial: &str,
    credentials: bool,
) -> Result<Option<(String, String)>, AppError> {
    let context = Context {
        title: title.into(),
        initial: initial.into(),
        credentials,
        result: RefCell::new(None),
    };
    let code = unsafe {
        DialogBoxParamW(
            Some(platform::instance()?),
            resource(if credentials {
                CREDENTIAL_DIALOG
            } else {
                QUEUE_NAME_DIALOG
            }),
            Some(owner),
            Some(dialog),
            LPARAM((&context as *const Context) as isize),
        )
    };
    if code == -1 {
        return Err(AppError("Cannot open queue dialog.".into()));
    }
    Ok(context.result.into_inner())
}
unsafe extern "system" fn dialog(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> isize {
    unsafe {
        if msg == WM_INITDIALOG {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, l.0);
            let context = &*(l.0 as *const Context);
            let title = wide(&context.title);
            let _ = SetWindowTextW(hwnd, PCWSTR(title.as_ptr()));
            let id = if context.credentials {
                LOGIN_EDIT
            } else {
                QUEUE_NAME_EDIT
            };
            if let Ok(edit) = item(hwnd, id) {
                let _ = set_text(edit, &context.initial);
                platform::send(
                    edit,
                    EM_SETLIMITTEXT,
                    if context.credentials { 4096 } else { 160 },
                    0,
                );
            }
            platform::bring_to_front(hwnd);
            return 1;
        }
        let pointer = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Context;
        if pointer.is_null() {
            return 0;
        }
        let context = &*pointer;
        match msg {
            WM_COMMAND => match (w.0 & 0xffff) as u16 {
                1 => {
                    let value = |id| item(hwnd, id).map(text).unwrap_or_default();
                    *context.result.borrow_mut() = Some(if context.credentials {
                        (value(LOGIN_EDIT), value(PASSWORD_EDIT))
                    } else {
                        (value(QUEUE_NAME_EDIT), String::new())
                    });
                    let _ = EndDialog(hwnd, 1);
                    return 1;
                }
                2 => {
                    let _ = EndDialog(hwnd, 0);
                    return 1;
                }
                _ => {}
            },
            WM_CLOSE => {
                let _ = EndDialog(hwnd, 0);
                return 1;
            }
            WM_NCDESTROY => {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            }
            _ => {}
        }
        0
    }
}
