//! Private, current-user encrypted state. No request credentials enter this schema.
use crate::*;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct StoredQueue {
    pub id: QueueId,
    pub name: String,
    pub parallelism: u8,
    pub entries: Vec<EntryId>,
    #[serde(default)]
    pub revision: u64,
    #[serde(default)]
    pub schedule: crate::schedule::ScheduleRecord,
}
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Checkpoint {
    #[serde(default)]
    pub creation_time: Option<std::time::SystemTime>,
    pub partial: PathBuf,
    pub identity: DestinationIdentity,
    pub durable: u64,
    pub prefix_hash: Vec<u8>,
    pub bytes_hash: Vec<u8>,
    pub total: Option<u64>,
    pub response_url: Option<String>,
    pub etag: Option<String>,
}
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Journal {
    pub checkpoint: Checkpoint,
    pub original: Option<(PathBuf, DestinationIdentity)>,
}
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct StoredEntry {
    #[serde(default)]
    pub options: DownloadOptions,
    #[serde(default)]
    pub ephemeral: bool,
    #[serde(default)]
    pub staging: Option<Checkpoint>,
    pub id: EntryId,
    pub url: String,
    pub destination: PathBuf,
    pub category: Option<String>,
    pub description: String,
    pub authenticated: bool,
    pub completed: bool,
    #[serde(default)]
    pub completed_bytes: Option<u64>,
    #[serde(default)]
    pub retained_files: Vec<PathBuf>,
    pub checkpoint: Option<Checkpoint>,
    pub journal: Option<Journal>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct State {
    version: u32,
    pub next_entry: u64,
    pub next_queue: u64,
    pub queues: BTreeMap<QueueId, StoredQueue>,
    pub entries: BTreeMap<EntryId, StoredEntry>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            version: 3,
            next_entry: 1,
            next_queue: 2,
            queues: [(
                QueueId::MAIN,
                StoredQueue {
                    id: QueueId::MAIN,
                    name: "Main download queue".into(),
                    parallelism: 1,
                    entries: vec![],
                    revision: 0,
                    schedule: Default::default(),
                },
            )]
            .into(),
            entries: BTreeMap::new(),
        }
    }
}
pub(crate) struct Store {
    path: Option<PathBuf>,
    state: Mutex<State>,
    blocked: Option<String>,
}
#[derive(Clone)]
pub(crate) struct Recovery {
    pub store: Arc<Store>,
    pub entry: EntryId,
}
impl Store {
    pub fn open(durable: bool) -> Arc<Self> {
        if !durable {
            return Arc::new(Self {
                path: None,
                state: Mutex::new(State::default()),
                blocked: None,
            });
        }
        let path = std::env::var_os("LOCALAPPDATA")
            .map(|p| PathBuf::from(p).join("RustyIDM").join("queues.v1.dpapi"));
        let loaded = path
            .as_ref()
            .ok_or_else(|| "LocalAppData is unavailable.".to_string())
            .and_then(|p| match std::fs::read(p) {
                Ok(bytes) => {
                    if bytes.len() > 16 * 1024 * 1024 {
                        return Err("Queue storage exceeds the supported size.".into());
                    }
                    let plain = crypt(&bytes, false)?;
                    let mut state: State = serde_json::from_slice(&plain).map_err(|_| {
                        "Queue storage is corrupt or incompatible. The original was preserved."
                            .to_string()
                    })?;
                    validate(&state)?;
                    state.version = 3;
                    Ok(state)
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(State::default()),
                Err(_) => Err("Queue storage cannot be read. The original was preserved.".into()),
            });
        let (state, blocked) = match loaded {
            Ok(s) => (s, None),
            Err(e) => (State::default(), Some(e)),
        };
        Arc::new(Self {
            path,
            state: Mutex::new(state),
            blocked,
        })
    }
    pub fn problem(&self) -> Option<&str> {
        self.blocked.as_deref()
    }
    pub fn snapshot(&self) -> State {
        self.state.lock().unwrap_or_else(|p| p.into_inner()).clone()
    }
    pub fn update<T>(
        &self,
        action: impl FnOnce(&mut State) -> Result<T, String>,
    ) -> Result<T, String> {
        if let Some(error) = &self.blocked {
            return Err(error.clone());
        }
        let mut guard = self
            .state
            .lock()
            .map_err(|_| "Queue storage lock failed.".to_string())?;
        let mut next = guard.clone();
        let result = action(&mut next)?;
        validate(&next)?;
        if let Some(path) = &self.path {
            let mut plain =
                serde_json::to_vec(&next).map_err(|_| "Cannot encode queue state.".to_string())?;
            let encrypted = crypt(&plain, true);
            plain.fill(0);
            let encrypted = encrypted?;
            if encrypted.len() > 16 * 1024 * 1024 {
                return Err(
                    "Queue storage exceeds the supported size; the previous state is preserved."
                        .into(),
                );
            }
            atomic_write(path, &encrypted)?;
        }
        *guard = next;
        Ok(result)
    }
}
impl Recovery {
    pub fn checkpoint(&self) -> Option<Checkpoint> {
        self.store
            .snapshot()
            .entries
            .get(&self.entry)
            .and_then(|e| e.checkpoint.clone())
    }
    pub fn save(&self, checkpoint: Checkpoint) -> Result<(), JobError> {
        self.store
            .update(|s| {
                let entry = s
                    .entries
                    .get_mut(&self.entry)
                    .ok_or("Recovery entry was removed.")?;
                entry.checkpoint = Some(checkpoint);
                Ok(())
            })
            .map_err(storage_error)
    }
    pub fn journal(
        &self,
        checkpoint: Checkpoint,
        original: Option<(PathBuf, DestinationIdentity)>,
    ) -> Result<(), JobError> {
        self.store
            .update(|s| {
                let entry = s
                    .entries
                    .get_mut(&self.entry)
                    .ok_or("Recovery entry was removed.")?;
                entry.journal = Some(Journal {
                    checkpoint,
                    original,
                });
                Ok(())
            })
            .map_err(storage_error)
    }
    pub fn finish(&self) -> Result<(), JobError> {
        self.store
            .update(|s| {
                let entry = s
                    .entries
                    .get_mut(&self.entry)
                    .ok_or("Recovery entry was removed.")?;
                entry.completed_bytes = entry.checkpoint.as_ref().map(|c| c.durable);
                entry.completed = true;
                entry.checkpoint = None;
                entry.journal = None;
                Ok(())
            })
            .map_err(storage_error)
    }
    pub fn clear(&self) -> Result<(), JobError> {
        self.store
            .update(|s| {
                if let Some(e) = s.entries.get_mut(&self.entry) {
                    e.checkpoint = None;
                    e.journal = None;
                    e.staging = None;
                    e.completed = false;
                    e.completed_bytes = None;
                }
                Ok(())
            })
            .map_err(storage_error)
    }
}
pub(crate) fn storage_error(message: String) -> JobError {
    JobError::new(
        ErrorKind::Storage,
        format!("Queue storage failed; the queue is held. {message}"),
    )
}
fn validate(s: &State) -> Result<(), String> {
    if !matches!(s.version, 1 | 2 | 3)
        || !s.queues.contains_key(&QueueId::MAIN)
        || s.entries.len() > 1000
        || s.queues.len() > 100
    {
        return Err(
            "Queue storage has an unsupported schema or entry count; it was preserved.".into(),
        );
    }
    let mut members = std::collections::BTreeSet::new();
    for (id, q) in &s.queues {
        q.schedule.config.validate()?;
        if q.schedule.last_fired.is_some_and(|at| !at.valid()) {
            return Err("Invalid scheduled occurrence; storage was preserved.".into());
        }
        if *id != q.id
            || !(1..=2).contains(&q.parallelism)
            || q.name.trim().is_empty()
            || q.name.chars().any(char::is_control)
            || q.name.chars().count() > 160
        {
            return Err("Invalid queue definition; storage was preserved.".into());
        }
        for e in &q.entries {
            if !s.entries.contains_key(e) || !members.insert(*e) {
                return Err("Invalid queue membership; storage was preserved.".into());
            }
        }
    }
    for (id, e) in &s.entries {
        e.options.validate()?;
        crate::validate_request(
            &e.url,
            e.destination.clone(),
            crate::ReplacementPolicy::Deny,
        )
        .map_err(|_| "Invalid stored request; storage was preserved.")?;
        let u = url::Url::parse(&e.url).map_err(|_| "Invalid stored address.")?;
        if *id != e.id
            || id.0 >= s.next_entry
            || !matches!(u.scheme(), "http" | "https")
            || !u.username().is_empty()
            || u.password().is_some()
        {
            return Err("Invalid stored entry; storage was preserved.".into());
        }
    }
    if s.queues.keys().any(|id| id.0 >= s.next_queue) {
        return Err("Invalid queue identifier counter.".into());
    }
    Ok(())
}
fn atomic_write(path: &std::path::Path, bytes: &[u8]) -> Result<(), String> {
    use std::io::Write;
    let parent = path.parent().ok_or("Invalid queue storage folder.")?;
    std::fs::create_dir_all(parent).map_err(|_| "Cannot create the queue storage folder.")?;
    static NEXT_TEMP: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    let serial = NEXT_TEMP.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let temp = path.with_extension(format!("{}-{serial}.tmp", std::process::id()));
    let mut reserved = false;
    let result = (|| {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
            .map_err(|_| "Cannot reserve queue storage replacement.")?;
        reserved = true;
        file.write_all(bytes)
            .and_then(|_| file.sync_all())
            .map_err(|_| "Cannot sync queue storage replacement.")?;
        drop(file);
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStrExt;
            use windows::{
                core::PCWSTR,
                Win32::Storage::FileSystem::{
                    MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
                },
            };
            let a: Vec<u16> = temp.as_os_str().encode_wide().chain(Some(0)).collect();
            let b: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
            unsafe {
                MoveFileExW(
                    PCWSTR(a.as_ptr()),
                    PCWSTR(b.as_ptr()),
                    MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
                )
            }
            .map_err(|_| "Cannot atomically replace queue storage.")?;
        }
        #[cfg(not(windows))]
        {
            std::fs::rename(&temp, path).map_err(|_| "Cannot replace queue storage.")?;
        }
        Ok(())
    })();
    if result.is_err() && reserved {
        let _ = std::fs::remove_file(&temp);
    }
    result.map_err(str::to_owned)
}
#[cfg(windows)]
fn crypt(input: &[u8], encrypt: bool) -> Result<Vec<u8>, String> {
    use windows::{
        core::PCWSTR,
        Win32::{
            Foundation::{LocalFree, HLOCAL},
            Security::Cryptography::*,
        },
    };
    let source = CRYPT_INTEGER_BLOB {
        cbData: input
            .len()
            .try_into()
            .map_err(|_| "Queue state is too large.")?,
        pbData: input.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB::default();
    unsafe {
        let result = if encrypt {
            CryptProtectData(
                &source,
                PCWSTR::null(),
                None,
                None,
                None,
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        } else {
            CryptUnprotectData(
                &source,
                None,
                None,
                None,
                None,
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        };
        result.map_err(|_| "Windows could not encrypt/decrypt queue storage for this user. The original is preserved.".to_string())?;
        let bytes = std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
        std::ptr::write_bytes(output.pbData, 0, output.cbData as usize);
        let _ = LocalFree(Some(HLOCAL(output.pbData.cast())));
        Ok(bytes)
    }
}
#[cfg(not(windows))]
fn crypt(_: &[u8], _: bool) -> Result<Vec<u8>, String> {
    Err("Durable queues require Windows current-user encryption.".into())
}
