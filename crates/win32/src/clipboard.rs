//! Current-session clipboard subscription; never consumes pre-existing text.
use windows::Win32::{
    Foundation::{HGLOBAL, HWND},
    System::{DataExchange::*, Memory::*},
};
pub struct Clipboard {
    hwnd: HWND,
    enabled: bool,
    sequence: u32,
}
impl Clipboard {
    pub fn new(hwnd: HWND) -> Self {
        Self {
            hwnd,
            enabled: false,
            sequence: 0,
        }
    }
    pub fn adopt(&mut self, enabled: bool) {
        self.enabled = enabled;
        self.sequence = unsafe { GetClipboardSequenceNumber() };
    }
    pub fn configure(&mut self, enabled: bool) -> Result<(), String> {
        if enabled == self.enabled {
            return Ok(());
        }
        unsafe {
            if enabled {
                self.sequence = GetClipboardSequenceNumber();
                AddClipboardFormatListener(self.hwnd)
                    .map_err(|_| "Cannot monitor the clipboard.".to_owned())?;
            } else {
                RemoveClipboardFormatListener(self.hwnd)
                    .map_err(|_| "Cannot remove the clipboard monitor.".to_owned())?;
            }
        }
        self.enabled = enabled;
        Ok(())
    }
    pub fn take_url(&mut self) -> Option<String> {
        if !self.enabled {
            return None;
        }
        unsafe {
            let sequence = GetClipboardSequenceNumber();
            if sequence == 0 || sequence == self.sequence {
                return None;
            }
            if OpenClipboard(Some(self.hwnd)).is_err() {
                return None;
            }
            struct Close;
            impl Drop for Close {
                fn drop(&mut self) {
                    unsafe {
                        let _ = CloseClipboard();
                    }
                }
            }
            let _close = Close;
            self.sequence = sequence;
            let data = GetClipboardData(13).ok()?;
            let memory = HGLOBAL(data.0);
            let bytes = GlobalSize(memory);
            if bytes == 0 || bytes > 32768 || bytes % 2 != 0 {
                return None;
            }
            let pointer = GlobalLock(memory);
            if pointer.is_null() {
                return None;
            }
            let slice = std::slice::from_raw_parts(pointer.cast::<u16>(), bytes / 2);
            let result = slice
                .iter()
                .position(|c| *c == 0)
                .and_then(|n| String::from_utf16(&slice[..n]).ok());
            let _ = GlobalUnlock(memory);
            let text = result?;
            if text.is_empty() || text.trim() != text || text.chars().any(char::is_whitespace) {
                return None;
            }
            let url = url::Url::parse(&text).ok()?;
            if !matches!(url.scheme(), "http" | "https")
                || url.host_str().is_none()
                || !url.username().is_empty()
                || url.password().is_some()
            {
                return None;
            }
            Some(text)
        }
    }
}
impl Drop for Clipboard {
    fn drop(&mut self) {
        if self.enabled {
            unsafe {
                let _ = RemoveClipboardFormatListener(self.hwnd);
            }
        }
    }
}

/// Copies only after an explicit product action; Windows owns the allocation
/// after SetClipboardData succeeds.
pub fn copy_text(owner: HWND, text: &str) -> Result<(), String> {
    use windows::Win32::Foundation::{GlobalFree, HANDLE};
    struct Allocation(Option<HGLOBAL>);
    impl Drop for Allocation {
        fn drop(&mut self) {
            if let Some(memory) = self.0 {
                unsafe {
                    let _ = GlobalFree(Some(memory));
                }
            }
        }
    }
    let value = crate::platform::wide(text);
    let memory = unsafe { GlobalAlloc(GMEM_MOVEABLE, value.len() * 2) }
        .map_err(|_| "Cannot allocate clipboard text.".to_owned())?;
    let mut allocation = Allocation(Some(memory));
    unsafe {
        let pointer = GlobalLock(memory);
        if pointer.is_null() {
            return Err("Cannot prepare clipboard text.".into());
        }
        std::ptr::copy_nonoverlapping(value.as_ptr(), pointer.cast::<u16>(), value.len());
        let _ = GlobalUnlock(memory);
        OpenClipboard(Some(owner)).map_err(|_| "The clipboard is busy. Try again.".to_owned())?;
        let result = EmptyClipboard().and_then(|_| SetClipboardData(13, Some(HANDLE(memory.0))));
        let _ = CloseClipboard();
        result.map_err(|_| "Windows could not copy the link.".to_owned())?;
        allocation.0 = None;
    }
    Ok(())
}
