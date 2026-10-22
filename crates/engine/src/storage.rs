//! Private, current-user encrypted state. No request credentials enter this schema.
use crate::history::{
    HistoryMetadata, HistoryOutcome, HISTORY_FULL, MAX_HISTORY_ENTRIES, MAX_STORAGE_BYTES,
};
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
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Checkpoint {
    #[serde(default)]
    pub refresh_pending: bool,
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
    #[serde(default)]
    pub last_modified: Option<String>,
    #[serde(default)]
    pub segments: Vec<SegmentCheckpoint>,
    #[serde(default)]
    pub cleanup_pending: Vec<PathBuf>,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SegmentCheckpoint {
    pub start: u64,
    pub end: u64,
    pub saved: u64,
    pub partial: PathBuf,
    pub identity: DestinationIdentity,
    pub prefix_hash: Vec<u8>,
    pub bytes_hash: Vec<u8>,
}
impl Checkpoint {
    pub fn received(&self) -> u64 {
        self.segments.iter().fold(self.durable, |total, s| {
            total.saturating_add(s.saved.saturating_sub(self.durable.saturating_sub(s.start)))
        })
    }
    pub fn segmented(&self) -> bool {
        !self.segments.is_empty()
    }
}
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Journal {
    pub checkpoint: Checkpoint,
    pub original: Option<(PathBuf, DestinationIdentity)>,
}
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct StoredEntry {
    #[serde(default)]
    pub history: HistoryMetadata,
    #[serde(default)]
    pub media: Option<crate::media::MediaSelection>,
    #[serde(default)]
    pub source_page: Option<String>,
    #[serde(default)]
    pub refresh_needs_capture: bool,
    #[serde(default)]
    pub options: DownloadOptions,
    #[serde(default)]
    pub ephemeral: bool,
    #[serde(default)]
    pub admission_pending: bool,
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
            version: 7,
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
    pending_history: Mutex<BTreeMap<EntryId, HistoryMetadata>>,
    blocked: Option<String>,
    #[cfg(test)]
    fail_writes: std::sync::atomic::AtomicBool,
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
                pending_history: Mutex::new(BTreeMap::new()),
                blocked: None,
                #[cfg(test)]
                fail_writes: std::sync::atomic::AtomicBool::new(false),
            });
        }
        let path = std::env::var_os("LOCALAPPDATA")
            .map(|p| PathBuf::from(p).join("RustyIDM").join("queues.v1.dpapi"));
        Self::open_path(path)
    }
    fn open_path(path: Option<PathBuf>) -> Arc<Self> {
        let loaded = path
            .as_ref()
            .ok_or_else(|| "LocalAppData is unavailable.".to_string())
            .and_then(|p| match std::fs::read(p) {
                Ok(bytes) => {
                    if bytes.len() > MAX_STORAGE_BYTES {
                        return Err("Queue storage exceeds the supported size.".into());
                    }
                    let mut plain = crypt(&bytes, false)?;
                    let state = decode_state(&plain);
                    plain.fill(0);
                    state
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
            pending_history: Mutex::new(BTreeMap::new()),
            blocked,
            #[cfg(test)]
            fail_writes: std::sync::atomic::AtomicBool::new(false),
        })
    }
    pub fn problem(&self) -> Option<&str> {
        self.blocked.as_deref()
    }
    pub(crate) fn history(&self, entry: EntryId) -> Option<HistoryMetadata> {
        let state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        let pending = self
            .pending_history
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        state
            .entries
            .get(&entry)
            .map(|saved| pending.get(&entry).unwrap_or(&saved.history).clone())
    }
    #[cfg(test)]
    pub(crate) fn set_write_failure_for_test(&self, fail: bool) {
        self.fail_writes
            .store(fail, std::sync::atomic::Ordering::SeqCst);
    }
    #[cfg(test)]
    pub(crate) fn reloaded_for_test(&self) -> Arc<Self> {
        let plain = serde_json::to_vec(&*self.state.lock().unwrap()).unwrap();
        let state: State = serde_json::from_slice(&plain).unwrap();
        validate(&state).unwrap();
        Arc::new(Self {
            path: None,
            state: Mutex::new(state),
            pending_history: Mutex::new(BTreeMap::new()),
            blocked: None,
            fail_writes: std::sync::atomic::AtomicBool::new(false),
        })
    }
    pub fn snapshot(&self) -> State {
        let guard = self.state.lock().unwrap_or_else(|p| p.into_inner());
        let pending = self
            .pending_history
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        let mut state = guard.clone();
        apply_pending(&mut state, &pending);
        state
    }
    pub fn has_pending_history(&self) -> bool {
        !self
            .pending_history
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .is_empty()
    }
    pub fn flush_history(&self) -> Result<(), String> {
        if self.has_pending_history() {
            self.update(|_| Ok(()))
        } else {
            Ok(())
        }
    }
    fn save_history(&self, entry: EntryId, history: HistoryMetadata) -> Result<(), String> {
        {
            let state = self
                .state
                .lock()
                .map_err(|_| "Download storage lock failed.")?;
            if !state.entries.contains_key(&entry) {
                return Err("History entry is missing.".into());
            }
            self.pending_history
                .lock()
                .map_err(|_| "History metadata lock failed.")?
                .insert(entry, history);
        }
        self.update(|_| Ok(()))
    }
    pub fn update<T>(
        &self,
        action: impl FnOnce(&mut State) -> Result<T, String>,
    ) -> Result<T, String> {
        #[cfg(test)]
        if self.fail_writes.load(std::sync::atomic::Ordering::SeqCst) {
            return Err("Injected history write failure.".into());
        }
        if let Some(error) = &self.blocked {
            return Err(error.clone());
        }
        let mut guard = self
            .state
            .lock()
            .map_err(|_| "Queue storage lock failed.".to_string())?;
        let mut next = guard.clone();
        let mut pending = self
            .pending_history
            .lock()
            .map_err(|_| "History metadata lock failed.")?;
        apply_pending(&mut next, &pending);
        let result = action(&mut next)?;
        validate(&next)?;
        let mut plain =
            serde_json::to_vec(&next).map_err(|_| "Cannot encode download state.".to_string())?;
        if plain.len() > MAX_STORAGE_BYTES {
            plain.fill(0);
            return Err(format!(
                "{HISTORY_FULL} The 16 MiB storage limit was reached."
            ));
        }
        if let Some(path) = &self.path {
            let encrypted = crypt(&plain, true);
            plain.fill(0);
            let encrypted = encrypted?;
            if encrypted.len() > MAX_STORAGE_BYTES {
                return Err(format!(
                    "{HISTORY_FULL} The 16 MiB storage limit was reached."
                ));
            }
            atomic_write(path, &encrypted)?;
        }
        plain.fill(0);
        *guard = next;
        pending.clear();
        Ok(result)
    }
}
fn decode_state(plain: &[u8]) -> Result<State, String> {
    if plain.len() > MAX_STORAGE_BYTES {
        return Err(HISTORY_FULL.into());
    }
    let mut state: State = serde_json::from_slice(plain).map_err(|_| {
        "Queue storage is corrupt or incompatible. The original was preserved.".to_string()
    })?;
    validate(&state)?;
    state.version = 7;
    Ok(state)
}
impl Recovery {
    pub fn save_history(&self, history: HistoryMetadata) -> Result<(), JobError> {
        self.store
            .save_history(self.entry, history)
            .map_err(storage_error)
    }
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
    pub fn finish(&self, bytes: u64) -> Result<(), JobError> {
        let mut history = self
            .store
            .history(self.entry)
            .ok_or_else(|| storage_error("Recovery entry was removed.".into()))?;
        history.outcome = Some(HistoryOutcome::Completed);
        history.finished_at = Some(std::time::SystemTime::now());
        history.received = bytes;
        history.total = Some(bytes);
        history.error = None;
        self.save_history(history)
    }
    pub fn clear(&self) -> Result<(), JobError> {
        self.store
            .update(|s| {
                if let Some(e) = s.entries.get_mut(&self.entry) {
                    e.checkpoint = None;
                    e.refresh_needs_capture = false;
                    e.journal = None;
                    e.staging = None;
                }
                Ok(())
            })
            .map_err(storage_error)
    }
}
fn apply_pending(state: &mut State, pending: &BTreeMap<EntryId, HistoryMetadata>) {
    for (id, history) in pending {
        if let Some(entry) = state.entries.get_mut(id) {
            entry.history = history.clone();
            entry.completed = history.outcome == Some(HistoryOutcome::Completed);
            entry.completed_bytes = entry.completed.then_some(history.received);
            if entry.completed {
                entry.checkpoint = None;
                entry.journal = None;
            }
        }
    }
}
pub(crate) fn storage_error(message: String) -> JobError {
    JobError::new(
        ErrorKind::Storage,
        format!("Download storage failed; the download is held. {message}"),
    )
}
fn validate(s: &State) -> Result<(), String> {
    if s.entries.len()
        > if s.version >= 7 {
            MAX_HISTORY_ENTRIES
        } else {
            1000
        }
    {
        return Err(HISTORY_FULL.into());
    }
    if !matches!(s.version, 1 | 2 | 3 | 4 | 5 | 6 | 7)
        || !s.queues.contains_key(&QueueId::MAIN)
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
        if let Some(c) = &e.checkpoint {
            if c.segments.len() > 128
                || c.cleanup_pending.len() > 128
                || c.segments.iter().any(|segment| {
                    segment.start > segment.end
                        || segment
                            .end
                            .checked_sub(segment.start)
                            .and_then(|n| n.checked_add(1))
                            .is_none_or(|length| segment.saved > length)
                        || segment.partial == c.partial
                        || !segment.partial.is_absolute()
                        || segment.prefix_hash.len() != 32
                        || segment.bytes_hash.len() != 32
                })
                || c.cleanup_pending
                    .iter()
                    .any(|path| !c.segments.iter().any(|segment| &segment.partial == path))
            {
                return Err("Invalid saved range map; storage was preserved.".into());
            }
            if let Some(total) = c.total {
                if c.durable > total
                    || c.received() > total
                    || c.segments.iter().any(|segment| segment.end >= total)
                {
                    return Err(
                        "Saved ranges exceed the declared file size; storage was preserved.".into(),
                    );
                }
                if !c.segments.is_empty() {
                    let mut ordered: Vec<_> = c.segments.iter().collect();
                    ordered.sort_by_key(|segment| segment.start);
                    let mut next = ordered[0].start;
                    if next == 0
                        || ordered.iter().any(|segment| {
                            if segment.start != next {
                                return true;
                            }
                            next = segment.end.saturating_add(1);
                            false
                        })
                        || next != total
                    {
                        return Err(
                            "Saved ranges have a gap or overlap; storage was preserved.".into()
                        );
                    }
                }
            }
        }
        e.options.validate()?;
        if e.media
            .as_ref()
            .is_some_and(|selection| selection.validate().is_err())
        {
            return Err("Invalid saved media selection; storage was preserved.".into());
        }
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
        if let Some(page) = &e.source_page {
            let parsed = url::Url::parse(page).map_err(|_| "Invalid stored source page.")?;
            if page.len() > 8192
                || page.chars().any(char::is_control)
                || !matches!(parsed.scheme(), "http" | "https")
                || parsed.host_str().is_none()
                || !parsed.username().is_empty()
                || parsed.password().is_some()
            {
                return Err("Invalid stored source page.".into());
            }
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

#[cfg(test)]
#[path = "storage_history_tests.rs"]
mod history_tests;
