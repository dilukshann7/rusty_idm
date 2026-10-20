//! Shell calls run on a dedicated COM worker. Only copied image-list entries
//! reach controls; owned HICONs are destroyed even for stale worker results.
use crate::{
    controls::scale,
    platform::{send, wide},
    AppError,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    sync::{mpsc, Arc, Mutex},
};
use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::HWND,
        Storage::FileSystem::FILE_ATTRIBUTE_NORMAL,
        System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED},
        UI::{Controls::*, Shell::*, WindowsAndMessaging::*},
    },
};
pub const ASSOCIATIONS_CHANGED: u32 = WM_APP + 12;
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Key {
    Stock(i32),
    Extension(String),
    File(PathBuf, u128),
}
struct Icon(usize);
impl Drop for Icon {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyIcon(HICON(self.0 as *mut _));
        }
    }
}
struct ResultIcon {
    generation: u64,
    key: Key,
    icon: Option<Icon>,
}
pub struct ShellImages {
    list: HIMAGELIST,
    size: i32,
    generation: u64,
    sender: Option<mpsc::SyncSender<(u64, Key, i32)>>,
    results: Arc<Mutex<Vec<ResultIcon>>>,
    pending: BTreeSet<Key>,
    cached: BTreeMap<Key, i32>,
    registration: u32,
    tree: HWND,
    downloads: HWND,
}
impl ShellImages {
    pub fn new(owner: HWND, tree: HWND, downloads: HWND, dpi: u32) -> Result<Self, AppError> {
        let size = scale(16, dpi);
        let list = unsafe { ImageList_Create(size, size, ILC_COLOR32 | ILC_MASK, 16, 16) };
        if list.is_invalid() {
            return Err(AppError("Cannot create Windows file icon images.".into()));
        }
        let (sender, receiver) = mpsc::sync_channel::<(u64, Key, i32)>(128);
        let results = Arc::new(Mutex::new(Vec::new()));
        let output = results.clone();
        let worker = std::thread::Builder::new()
            .name("rusty-shell-icons".into())
            .spawn(move || {
                let apartment = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }.is_ok();
                if !apartment {
                    return;
                }
                while let Ok((generation, key, size)) = receiver.recv() {
                    let icon = lookup(&key, size);
                    let mut result = output.lock().unwrap_or_else(|p| p.into_inner());
                    if result.len() < 256 {
                        result.push(ResultIcon {
                            generation,
                            key,
                            icon,
                        });
                    }
                }
                unsafe {
                    CoUninitialize();
                }
            });
        if worker.is_err() {
            unsafe {
                let _ = ImageList_Destroy(Some(list));
            }
            return Err(AppError("Cannot start Windows icon lookup worker.".into()));
        }
        let entry = SHChangeNotifyEntry {
            pidl: std::ptr::null_mut(),
            fRecursive: false.into(),
        };
        let registration = unsafe {
            SHChangeNotifyRegister(
                owner,
                SHCNRF_ShellLevel,
                SHCNE_ASSOCCHANGED.0 as i32,
                ASSOCIATIONS_CHANGED,
                1,
                &entry,
            )
        };
        send(
            tree,
            TVM_SETIMAGELIST,
            TVSIL_NORMAL as usize,
            list.0 as isize,
        );
        send(
            downloads,
            LVM_SETIMAGELIST,
            LVSIL_SMALL as usize,
            list.0 as isize,
        );
        Ok(Self {
            list,
            size,
            generation: 0,
            sender: Some(sender),
            results,
            pending: BTreeSet::new(),
            cached: BTreeMap::new(),
            registration,
            tree,
            downloads,
        })
    }
    pub fn invalidate(&mut self, dpi: u32) -> Result<(), AppError> {
        let size = scale(16, dpi);
        let list = unsafe { ImageList_Create(size, size, ILC_COLOR32 | ILC_MASK, 16, 16) };
        if list.is_invalid() {
            return Err(AppError(
                "Cannot rebuild file icons for the display.".into(),
            ));
        }
        send(
            self.tree,
            TVM_SETIMAGELIST,
            TVSIL_NORMAL as usize,
            list.0 as isize,
        );
        send(
            self.downloads,
            LVM_SETIMAGELIST,
            LVSIL_SMALL as usize,
            list.0 as isize,
        );
        unsafe {
            let _ = ImageList_Destroy(Some(self.list));
        }
        self.list = list;
        self.size = size;
        self.generation = self.generation.wrapping_add(1);
        self.pending.clear();
        self.cached.clear();
        Ok(())
    }
    pub fn drain(&mut self) {
        // Bound a long-running tray session even across repeated redownloads.
        if self.cached.len() >= 2048 {
            let _ = self.invalidate((self.size as u32) * 6);
        }

        let results = std::mem::take(&mut *self.results.lock().unwrap_or_else(|p| p.into_inner()));
        for result in results {
            if result.generation != self.generation {
                continue;
            }
            self.pending.remove(&result.key);
            let index = if let Some(icon) = result.icon {
                unsafe { ImageList_ReplaceIcon(self.list, -1, HICON(icon.0 as *mut _)) }
            } else {
                -1
            };
            self.cached.insert(result.key, index);
        }
    }
    pub fn index(&mut self, key: Key) -> i32 {
        if let Some(index) = self.cached.get(&key) {
            return *index;
        }
        if !self.pending.contains(&key) {
            if self.sender.as_ref().is_some_and(|s| {
                s.try_send((self.generation, key.clone(), self.size))
                    .is_ok()
            }) {
                self.pending.insert(key);
            }
        }
        self.cached
            .get(&Key::Stock(SIID_DOCNOASSOC.0))
            .copied()
            .unwrap_or(-1)
    }
    pub fn stock(kind: Option<&str>) -> Key {
        Key::Stock(match kind {
            Some("Compressed") => SIID_ZIPFILE.0,
            Some("Documents") => SIID_DOCASSOC.0,
            Some("Music") => SIID_AUDIOFILES.0,
            Some("Programs") => SIID_APPLICATION.0,
            Some("Video") => SIID_VIDEOFILES.0,
            _ => SIID_FOLDER.0,
        })
    }
    pub fn file(job: &rusty_engine::JobSnapshot) -> Key {
        if job.state == rusty_engine::JobState::Completed {
            Key::File(
                job.destination.clone(),
                job.last_try
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map_or(0, |d| d.as_nanos()),
            )
        } else {
            Key::Extension(
                job.destination
                    .extension()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_ascii_lowercase(),
            )
        }
    }
}
impl Drop for ShellImages {
    fn drop(&mut self) {
        self.sender.take();
        unsafe {
            if self.registration != 0 {
                let _ = SHChangeNotifyDeregister(self.registration);
            }
            if IsWindow(Some(self.tree)).as_bool() {
                send(self.tree, TVM_SETIMAGELIST, TVSIL_NORMAL as usize, 0);
            }
            if IsWindow(Some(self.downloads)).as_bool() {
                send(self.downloads, LVM_SETIMAGELIST, LVSIL_SMALL as usize, 0);
            }
            let _ = ImageList_Destroy(Some(self.list));
        }
    }
}
fn lookup(key: &Key, size: i32) -> Option<Icon> {
    unsafe {
        let mut info = SHFILEINFOW::default();
        match key {
            Key::Stock(id) => {
                let mut stock = SHSTOCKICONINFO {
                    cbSize: std::mem::size_of::<SHSTOCKICONINFO>() as u32,
                    ..Default::default()
                };
                SHGetStockIconInfo(
                    SHSTOCKICONID(*id),
                    SHGSI_ICON
                        | if size > 16 {
                            SHGSI_LARGEICON
                        } else {
                            SHGSI_SMALLICON
                        },
                    &mut stock,
                )
                .ok()?;
                return (!stock.hIcon.0.is_null()).then_some(Icon(stock.hIcon.0 as usize));
            }
            Key::Extension(extension) => {
                let name = wide(if extension.is_empty() {
                    "unknown".into()
                } else {
                    format!("file.{extension}")
                });
                if SHGetFileInfoW(
                    PCWSTR(name.as_ptr()),
                    FILE_ATTRIBUTE_NORMAL,
                    Some(&mut info),
                    std::mem::size_of::<SHFILEINFOW>() as u32,
                    SHGFI_ICON
                        | if size > 16 {
                            SHGFI_LARGEICON
                        } else {
                            SHGFI_SMALLICON
                        }
                        | SHGFI_USEFILEATTRIBUTES,
                ) == 0
                {
                    return lookup(&Key::Stock(SIID_DOCNOASSOC.0), size);
                }
            }
            Key::File(path, _) => {
                let name = wide(path);
                if SHGetFileInfoW(
                    PCWSTR(name.as_ptr()),
                    FILE_ATTRIBUTE_NORMAL,
                    Some(&mut info),
                    std::mem::size_of::<SHFILEINFOW>() as u32,
                    SHGFI_ICON
                        | if size > 16 {
                            SHGFI_LARGEICON
                        } else {
                            SHGFI_SMALLICON
                        },
                ) == 0
                {
                    return lookup(&Key::Stock(SIID_DOCNOASSOC.0), size);
                }
            }
        }
        (!info.hIcon.0.is_null()).then_some(Icon(info.hIcon.0 as usize))
    }
}
