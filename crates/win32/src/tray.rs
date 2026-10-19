use crate::{commands::*, platform, AppError};
use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::{HWND, LPARAM, POINT, WPARAM},
        UI::{Shell::*, WindowsAndMessaging::*},
    },
};

pub const TRAY_CALLBACK: u32 = WM_APP + 40;
// shellapi.h: NIN_KEYSELECT = NIN_SELECT | NINF_KEY (0x1).
pub const TRAY_KEY_SELECT: u32 = NIN_SELECT | 1;

pub struct TrayIcon {
    owner: HWND,
    installed: bool,
}
impl TrayIcon {
    pub fn new(owner: HWND) -> Self {
        Self {
            owner,
            installed: false,
        }
    }
    fn data(&self) -> Result<NOTIFYICONDATAW, AppError> {
        let mut data = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: self.owner,
            uID: 1,
            uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP | NIF_SHOWTIP,
            uCallbackMessage: TRAY_CALLBACK,
            hIcon: platform::app_icon()?,
            ..Default::default()
        };
        for (slot, character) in data
            .szTip
            .iter_mut()
            .zip("Rusty Download Manager 0.1.0".encode_utf16())
        {
            *slot = character;
        }
        Ok(data)
    }
    pub fn install(&mut self) -> bool {
        let Ok(mut data) = self.data() else {
            return false;
        };
        self.installed = unsafe { Shell_NotifyIconW(NIM_ADD, &data).as_bool() };
        if self.installed {
            data.Anonymous.uVersion = NOTIFYICON_VERSION_4;
            if !unsafe { Shell_NotifyIconW(NIM_SETVERSION, &data).as_bool() } {
                unsafe {
                    let _ = Shell_NotifyIconW(NIM_DELETE, &data);
                }
                self.installed = false;
            }
        }
        self.installed
    }
    pub fn available(&self) -> bool {
        self.installed
    }
    pub fn remove(&mut self) {
        if let Ok(data) = self.data() {
            unsafe {
                let _ = Shell_NotifyIconW(NIM_DELETE, &data);
            }
        }
        self.installed = false;
    }
    pub fn show_menu(&self) -> Result<Option<u16>, AppError> {
        unsafe {
            let menu = CreatePopupMenu()?;
            let result = (|| -> Result<Option<u16>, AppError> {
                for (id, label) in [
                    (SHOW_MANAGER, "Open Rusty Download Manager"),
                    (ADD, "Add URL..."),
                    (OPTIONS, "Options..."),
                ] {
                    let text = platform::wide(label);
                    AppendMenuW(menu, MF_STRING, id as usize, PCWSTR(text.as_ptr()))?;
                }
                AppendMenuW(menu, MF_SEPARATOR, 0, None)?;
                let text = platform::wide("Exit");
                AppendMenuW(menu, MF_STRING, EXIT as usize, PCWSTR(text.as_ptr()))?;
                SetMenuDefaultItem(menu, SHOW_MANAGER as u32, 0)?;
                let mut point = POINT::default();
                GetCursorPos(&mut point)?;
                // A notification icon menu requires a foreground owner to dismiss
                // when the user clicks elsewhere. This does not show the manager.
                let _ = SetForegroundWindow(self.owner);
                let selected = TrackPopupMenu(
                    menu,
                    TPM_RIGHTBUTTON | TPM_RETURNCMD | TPM_NONOTIFY,
                    point.x,
                    point.y,
                    None,
                    self.owner,
                    None,
                )
                .0;
                let _ = PostMessageW(Some(self.owner), WM_NULL, WPARAM(0), LPARAM(0));
                Ok((selected != 0).then_some(selected as u16))
            })();
            let _ = DestroyMenu(menu);
            result
        }
    }
}
impl Drop for TrayIcon {
    fn drop(&mut self) {
        self.remove();
    }
}
