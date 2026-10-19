use std::sync::{Arc, Mutex};
use windows::Win32::{
    Foundation::{HWND, LPARAM, WPARAM},
    UI::WindowsAndMessaging::{PostMessageW, WM_APP},
};
pub const ENGINE_WAKE: u32 = WM_APP + 1;
pub const DETAIL_CANCEL: u32 = WM_APP + 2;
pub const HOUSEKEEPING_TIMER: usize = 1;

pub struct WakeTarget {
    target: Mutex<Option<usize>>,
    notify: Arc<dyn Fn(usize) + Send + Sync>,
}
impl WakeTarget {
    pub fn new() -> Self {
        Self::with_notifier(Arc::new(|target| unsafe {
            let _ = PostMessageW(
                Some(HWND(target as *mut _)),
                ENGINE_WAKE,
                WPARAM(0),
                LPARAM(0),
            );
        }))
    }
    fn with_notifier(notify: Arc<dyn Fn(usize) + Send + Sync>) -> Self {
        Self {
            target: Mutex::new(None),
            notify,
        }
    }
    pub fn attach(&self, hwnd: HWND) {
        *self.target.lock().unwrap_or_else(|p| p.into_inner()) = Some(hwnd.0 as usize);
    }
    pub fn wake(&self) {
        let target = self.target.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(hwnd) = *target {
            (self.notify)(hwnd);
        }
    }
    /// Serializes against the last PostMessage before the main HWND is destroyed.
    pub fn disable(&self) {
        *self.target.lock().unwrap_or_else(|p| p.into_inner()) = None;
    }
}
impl Default for WakeTarget {
    fn default() -> Self {
        Self::new()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn disabled_target_cannot_post_late_notifications() {
        let count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let calls = count.clone();
        let target = Arc::new(WakeTarget::with_notifier(Arc::new(move |_| {
            calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        })));
        target.attach(HWND(1usize as *mut _));
        target.wake();
        target.disable();
        let other = target.clone();
        std::thread::spawn(move || {
            for _ in 0..1000 {
                other.wake();
            }
        })
        .join()
        .unwrap();
        assert_eq!(count.load(std::sync::atomic::Ordering::SeqCst), 1);
    }
}
