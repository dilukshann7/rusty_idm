use crate::AppError;
use std::{ffi::OsStr, mem::size_of, os::windows::ffi::OsStrExt};
use windows::{
    core::{w, PCWSTR},
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, WPARAM},
        Graphics::Gdi::{CreateFontIndirectW, DeleteObject, HFONT},
        System::{
            Com::{
                CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE,
            },
            LibraryLoader::GetModuleHandleW,
        },
        UI::{HiDpi::SystemParametersInfoForDpi, WindowsAndMessaging::*},
    },
};

pub fn wide(text: impl AsRef<OsStr>) -> Vec<u16> {
    text.as_ref().encode_wide().chain(Some(0)).collect()
}
pub fn resource(id: u16) -> PCWSTR {
    PCWSTR(id as usize as *const u16)
}
pub fn app_icon() -> windows::core::Result<HICON> {
    unsafe { LoadIconW(Some(instance()?), resource(crate::commands::APP_ICON)) }
}
pub fn instance() -> windows::core::Result<windows::Win32::Foundation::HINSTANCE> {
    unsafe { Ok(GetModuleHandleW(None)?.into()) }
}
pub fn send(hwnd: HWND, msg: u32, wparam: usize, lparam: isize) -> LRESULT {
    unsafe { SendMessageW(hwnd, msg, Some(WPARAM(wparam)), Some(LPARAM(lparam))) }
}
pub fn set_text(hwnd: HWND, text: &str) {
    let text = wide(text);
    unsafe {
        let _ = SetWindowTextW(hwnd, PCWSTR(text.as_ptr()));
    }
}
pub fn text(hwnd: HWND) -> String {
    unsafe {
        let len = GetWindowTextLengthW(hwnd).max(0) as usize;
        let mut buffer = vec![0; len + 1];
        let copied = GetWindowTextW(hwnd, &mut buffer);
        String::from_utf16_lossy(&buffer[..copied.max(0) as usize])
    }
}
pub fn item(dialog: HWND, id: i32) -> windows::core::Result<HWND> {
    unsafe { GetDlgItem(Some(dialog), id) }
}
/// Called on the GUI thread for an explicit browser handoff or activation.
pub fn bring_to_front(hwnd: HWND) {
    unsafe {
        let _ = ShowWindow(
            hwnd,
            if IsIconic(hwnd).as_bool() {
                SW_RESTORE
            } else {
                SW_SHOW
            },
        );
        let _ = BringWindowToTop(hwnd);
        let _ = SetForegroundWindow(hwnd);
    }
}
pub fn message(owner: Option<HWND>, title: &str, text: &str, confirm: bool) -> bool {
    let title = wide(title);
    let text = wide(text);
    unsafe {
        MessageBoxW(
            owner,
            PCWSTR(text.as_ptr()),
            PCWSTR(title.as_ptr()),
            if confirm {
                MB_YESNO | MB_ICONQUESTION | MB_DEFBUTTON2
            } else {
                MB_OK | MB_ICONINFORMATION
            },
        ) == IDYES
    }
}
pub struct ComApartment;
impl ComApartment {
    pub fn new() -> Result<Self, AppError> {
        unsafe {
            CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE).ok()?;
        }
        Ok(Self)
    }
}
impl Drop for ComApartment {
    fn drop(&mut self) {
        unsafe {
            CoUninitialize();
        }
    }
}

pub struct Font(pub HFONT);
impl Font {
    pub fn new(dpi: u32) -> Result<Self, AppError> {
        let mut metrics = NONCLIENTMETRICSW {
            cbSize: size_of::<NONCLIENTMETRICSW>() as u32,
            ..Default::default()
        };
        unsafe {
            SystemParametersInfoForDpi(
                SPI_GETNONCLIENTMETRICS.0,
                metrics.cbSize,
                Some((&mut metrics as *mut NONCLIENTMETRICSW).cast()),
                0,
                dpi,
            )?;
            // Match the compact classic utility/dialog metrics used by the
            // reference. Caption/menu sizing remains owned by Windows.
            metrics.lfMessageFont.lfHeight = -((dpi as i32 * 8 + 36) / 72);
            metrics.lfMessageFont.lfFaceName.fill(0);
            let face = wide("Tahoma");
            metrics.lfMessageFont.lfFaceName[..face.len()].copy_from_slice(&face);
            let font = CreateFontIndirectW(&metrics.lfMessageFont);
            if font.is_invalid() {
                return Err(windows::core::Error::from_thread().into());
            }
            Ok(Self(font))
        }
    }
    pub fn apply(&self, hwnd: HWND) {
        send(hwnd, WM_SETFONT, self.0 .0 as usize, 1);
    }
}
impl Drop for Font {
    fn drop(&mut self) {
        unsafe {
            let _ = DeleteObject(self.0.into());
        }
    }
}

pub fn error_title() -> PCWSTR {
    w!("Rusty Download Manager")
}
