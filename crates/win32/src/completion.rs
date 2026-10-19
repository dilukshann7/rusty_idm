use crate::{
    commands::*,
    platform::{self, item, resource, set_text},
    AppError,
};
use rusty_engine::{JobId, JobSnapshot};
use std::{cell::Cell, collections::BTreeMap, path::PathBuf};
use windows::{
    core::{w, PCWSTR},
    Win32::{
        Foundation::{ERROR_CANCELLED, HWND, LPARAM, WPARAM},
        UI::{
            Controls::{BST_CHECKED, BST_UNCHECKED, EM_SETLIMITTEXT},
            Shell::{
                SHGetStockIconInfo, SHOpenWithDialog, OAIF_EXEC, OPENASINFO, SHGSI_ICON,
                SHGSI_LARGEICON, SHSTOCKICONINFO, SIID_FOLDEROPEN,
            },
            WindowsAndMessaging::*,
        },
    },
};

#[derive(Clone, Copy)]
pub enum OpenAction {
    Open,
    OpenWith,
    OpenFolder,
}
pub enum CompletionAction {
    Open {
        owner: HWND,
        path: PathBuf,
        action: OpenAction,
    },
    ShowAgain(bool),
}
struct CompleteWindow {
    hwnd: Cell<HWND>,
    icon: Cell<HICON>,
    snapshot: JobSnapshot,
    show_again: bool,
    pending_open: Cell<Option<OpenAction>>,
    pending_preference: Cell<Option<bool>>,
}
impl Drop for CompleteWindow {
    fn drop(&mut self) {
        unsafe {
            if !self.hwnd.get().is_invalid() {
                let _ = DestroyWindow(self.hwnd.get());
            }
            if !self.icon.get().is_invalid() {
                let _ = DestroyIcon(self.icon.get());
            }
        }
    }
}
#[derive(Default)]
pub struct CompletionWindows {
    windows: BTreeMap<JobId, Box<CompleteWindow>>,
}
impl CompletionWindows {
    pub fn show(&mut self, owner: HWND, snapshot: &JobSnapshot) -> Result<(), AppError> {
        // A retry may finish while its previous completion window is still open.
        self.windows.remove(&snapshot.id);
        platform::bring_to_front(owner);
        let context = Box::new(CompleteWindow {
            hwnd: Cell::new(HWND::default()),
            icon: Cell::new(HICON::default()),
            snapshot: snapshot.clone(),
            show_again: true,
            pending_open: Cell::new(None),
            pending_preference: Cell::new(None),
        });
        unsafe {
            CreateDialogParamW(
                Some(platform::instance()?),
                resource(COMPLETE_DIALOG),
                Some(owner),
                Some(dialog_proc),
                LPARAM((&*context as *const CompleteWindow) as isize),
            )?;
        }
        self.windows.insert(snapshot.id, context);
        Ok(())
    }
    pub fn is_dialog_message(&self, msg: &MSG) -> bool {
        self.windows.values().any(|window| {
            !window.hwnd.get().is_invalid()
                && unsafe { IsDialogMessageW(window.hwnd.get(), msg).as_bool() }
        })
    }
    pub fn take_actions(&self) -> Vec<CompletionAction> {
        let mut actions = Vec::new();
        for window in self.windows.values() {
            if let Some(show) = window.pending_preference.take() {
                actions.push(CompletionAction::ShowAgain(show));
            }
            if let Some(action) = window.pending_open.take() {
                if !window.hwnd.get().is_invalid() {
                    actions.push(CompletionAction::Open {
                        owner: window.hwnd.get(),
                        path: window.snapshot.destination.clone(),
                        action,
                    });
                }
            }
        }
        actions
    }
    pub fn set_show_again(&self, show: bool) {
        for window in self.windows.values() {
            if let Ok(control) = item(window.hwnd.get(), COMPLETE_HIDE) {
                platform::send(
                    control,
                    BM_SETCHECK,
                    if show { BST_UNCHECKED.0 } else { BST_CHECKED.0 } as usize,
                    0,
                );
            }
        }
    }
    pub fn prune(&mut self) {
        self.windows
            .retain(|_, window| !window.hwnd.get().is_invalid());
    }
    pub fn close_all(&mut self) {
        self.windows.clear();
    }
}
unsafe extern "system" fn dialog_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> isize {
    std::panic::catch_unwind(|| unsafe { dialog_inner(hwnd, msg, wparam, lparam) }).unwrap_or(0)
}
unsafe fn dialog_inner(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> isize {
    unsafe {
        if msg == WM_INITDIALOG {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, lparam.0);
            let context = &*(lparam.0 as *const CompleteWindow);
            context.hwnd.set(hwnd);
            let hundredths = context.snapshot.received as u128 * 100 / 1_048_576;
            value(
                hwnd,
                COMPLETE_SUMMARY,
                &format!(
                    "Download complete\r\nDownloaded {}.{:02} MB ({} Bytes)",
                    hundredths / 100,
                    hundredths % 100,
                    context.snapshot.received
                ),
            );
            value(hwnd, COMPLETE_URL, &context.snapshot.display_url);
            value(
                hwnd,
                COMPLETE_PATH,
                &context.snapshot.destination.to_string_lossy(),
            );
            for id in [COMPLETE_URL, COMPLETE_PATH] {
                if let Ok(control) = item(hwnd, id) {
                    platform::send(control, EM_SETLIMITTEXT, 32767, 0);
                }
            }
            if let Ok(control) = item(hwnd, COMPLETE_HIDE) {
                platform::send(
                    control,
                    BM_SETCHECK,
                    if context.show_again {
                        BST_UNCHECKED.0
                    } else {
                        BST_CHECKED.0
                    } as usize,
                    0,
                );
            }
            let mut icon = SHSTOCKICONINFO {
                cbSize: std::mem::size_of::<SHSTOCKICONINFO>() as u32,
                ..Default::default()
            };
            if SHGetStockIconInfo(SIID_FOLDEROPEN, SHGSI_ICON | SHGSI_LARGEICON, &mut icon).is_ok()
            {
                context.icon.set(icon.hIcon);
                if let Ok(control) = item(hwnd, COMPLETE_ICON) {
                    platform::send(control, STM_SETICON, icon.hIcon.0 as usize, 0);
                }
            }
            platform::bring_to_front(hwnd);
            return 1;
        }
        let pointer = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const CompleteWindow;
        if pointer.is_null() {
            return 0;
        }
        let context = &*pointer;
        match msg {
            WM_COMMAND => match (wparam.0 & 0xffff) as i32 {
                COMPLETE_OPEN => {
                    context.pending_open.set(Some(OpenAction::Open));
                    1
                }
                COMPLETE_OPEN_WITH => {
                    context.pending_open.set(Some(OpenAction::OpenWith));
                    1
                }
                COMPLETE_FOLDER => {
                    context.pending_open.set(Some(OpenAction::OpenFolder));
                    1
                }
                COMPLETE_HIDE => {
                    let hide = item(hwnd, COMPLETE_HIDE).is_ok_and(|control| {
                        platform::send(control, BM_GETCHECK, 0, 0).0 == BST_CHECKED.0 as isize
                    });
                    context.pending_preference.set(Some(!hide));
                    1
                }
                2 => {
                    let _ = DestroyWindow(hwnd);
                    1
                }
                _ => 0,
            },
            WM_CLOSE => {
                let _ = DestroyWindow(hwnd);
                1
            }
            WM_NCDESTROY => {
                context.hwnd.set(HWND::default());
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                0
            }
            _ => 0,
        }
    }
}
fn value(hwnd: HWND, id: i32, text: &str) {
    if let Ok(control) = item(hwnd, id) {
        set_text(control, text);
    }
}

/// User actions run outside dialog callbacks so nested shell UI cannot borrow AppState.
pub fn open_completed(
    owner: HWND,
    path: &std::path::Path,
    action: OpenAction,
) -> Result<(), AppError> {
    if matches!(action, OpenAction::OpenFolder) {
        let folder = path
            .parent()
            .filter(|p| p.is_dir())
            .ok_or_else(|| AppError("The download folder is no longer available.".into()))?;
        return shell_open(owner, folder);
    }
    if !path.is_file() {
        return Err(AppError(
            "The downloaded file is no longer available.".into(),
        ));
    }
    if matches!(action, OpenAction::OpenWith) {
        let path = platform::wide(path);
        let info = OPENASINFO {
            pcszFile: PCWSTR(path.as_ptr()),
            pcszClass: PCWSTR::null(),
            oaifInFlags: OAIF_EXEC,
        };
        if let Err(error) = unsafe { SHOpenWithDialog(Some(owner), &info) } {
            if error.code() != windows::core::HRESULT::from_win32(ERROR_CANCELLED.0) {
                return Err(error.into());
            }
        }
        Ok(())
    } else {
        shell_open(owner, path)
    }
}
fn shell_open(owner: HWND, path: &std::path::Path) -> Result<(), AppError> {
    let path = platform::wide(path);
    let result = unsafe {
        windows::Win32::UI::Shell::ShellExecuteW(
            Some(owner),
            w!("open"),
            PCWSTR(path.as_ptr()),
            None,
            None,
            SW_SHOWNORMAL,
        )
    };
    if result.0 as isize <= 32 {
        return Err(AppError(
            "Windows could not open the downloaded file or folder.".into(),
        ));
    }
    Ok(())
}
