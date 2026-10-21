use crate::{commands::*, platform, AppError};
use std::cell::RefCell;
use windows::{
    core::{w, PCWSTR},
    Win32::{
        Foundation::{HWND, LPARAM, WPARAM},
        UI::{Controls::EM_SETLIMITTEXT, Shell::ShellExecuteW, WindowsAndMessaging::*},
    },
};

pub enum RefreshChoice {
    WaitForBrowser,
    Paste(String),
    Cancel,
}
struct Context {
    source_page: Option<String>,
    result: RefCell<Option<RefreshChoice>>,
}
pub fn choose(owner: HWND, source_page: Option<String>) -> Result<RefreshChoice, AppError> {
    let context = Context {
        source_page,
        result: RefCell::new(None),
    };
    let code = unsafe {
        DialogBoxParamW(
            Some(platform::instance()?),
            platform::resource(REFRESH_DIALOG),
            Some(owner),
            Some(dialog),
            LPARAM((&context as *const Context) as isize),
        )
    };
    if code == -1 {
        return Err(AppError("Cannot open the refresh dialog.".into()));
    }
    Ok(context.result.into_inner().unwrap_or(RefreshChoice::Cancel))
}
unsafe extern "system" fn dialog(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> isize {
    unsafe {
        if msg == WM_INITDIALOG {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, l.0);
            let context = &*(l.0 as *const Context);
            if let Ok(edit) = platform::item(hwnd, REFRESH_URL_EDIT) {
                platform::send(edit, EM_SETLIMITTEXT, 8192, 0);
            }
            if context.source_page.is_none() {
                if let Ok(button) = platform::item(hwnd, REFRESH_OPEN_PAGE as i32) {
                    let _ = EnableWindow(button, false);
                }
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
                REFRESH_OPEN_PAGE => {
                    if let Some(page) = &context.source_page {
                        let page = platform::wide(page);
                        let result = ShellExecuteW(
                            Some(hwnd),
                            w!("open"),
                            PCWSTR(page.as_ptr()),
                            None,
                            None,
                            SW_SHOWNORMAL,
                        );
                        if result.0 as isize <= 32 {
                            platform::message(
                                Some(hwnd),
                                "Source page",
                                "Windows could not open the source page.",
                                false,
                            );
                            return 1;
                        }
                        *context.result.borrow_mut() = Some(RefreshChoice::WaitForBrowser);
                        let _ = EndDialog(hwnd, 1);
                    }
                    return 1;
                }
                REFRESH_WAIT_BROWSER => {
                    *context.result.borrow_mut() = Some(RefreshChoice::WaitForBrowser);
                    let _ = EndDialog(hwnd, 1);
                    return 1;
                }
                1 => {
                    let url = platform::item(hwnd, REFRESH_URL_EDIT)
                        .map(platform::text)
                        .unwrap_or_default();
                    if url.trim().is_empty() {
                        platform::message(
                            Some(hwnd),
                            "Refresh address",
                            "Paste the new download address first.",
                            false,
                        );
                    } else {
                        *context.result.borrow_mut() =
                            Some(RefreshChoice::Paste(url.trim().to_owned()));
                        let _ = EndDialog(hwnd, 1);
                    }
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
