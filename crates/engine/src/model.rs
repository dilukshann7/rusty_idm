use std::{fmt, path::PathBuf, time::SystemTime};
use url::Url;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct JobId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JobState {
    Waiting,
    Running,
    Pausing,
    Paused,
    Cancelling,
    Completed,
    Failed,
    Cancelled,
}
impl JobState {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }
    pub fn can_retry(self) -> bool {
        matches!(self, Self::Failed | Self::Cancelled)
    }
    pub fn is_active(self) -> bool {
        !self.is_terminal()
    }
}

/// Opaque approval token. Never equate approval with permission to remove a target.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DestinationIdentity {
    pub(crate) size: u64,
    pub(crate) modified: Option<SystemTime>,
    #[cfg(windows)]
    pub(crate) volume: u32,
    #[cfg(windows)]
    pub(crate) index: u64,
}
#[derive(Clone, Debug)]
pub enum ReplacementPolicy {
    Deny,
    Approved(DestinationIdentity),
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DownloadOptions {
    pub user_agent: String,
    pub temporary_directory: Option<PathBuf>,
    pub server_creation_time: bool,
    pub ignore_modified_time: bool,
    pub confirmation_required: bool,
}
impl Default for DownloadOptions {
    fn default() -> Self {
        Self {
            user_agent: "RustyDownloadManager/0.1.0".into(),
            temporary_directory: None,
            server_creation_time: false,
            ignore_modified_time: false,
            confirmation_required: false,
        }
    }
}
impl DownloadOptions {
    pub fn validate(&self) -> Result<(), String> {
        if self.user_agent.is_empty()
            || self.user_agent.len() > 1024
            || self
                .user_agent
                .chars()
                .any(|c| c.is_control() || !c.is_ascii())
        {
            return Err("User-Agent must contain 1-1024 printable ASCII characters.".into());
        }
        if self
            .temporary_directory
            .as_ref()
            .is_some_and(|p| !p.is_absolute())
        {
            return Err("Choose an absolute temporary directory.".into());
        }
        Ok(())
    }
}
#[derive(Clone)]
pub struct DownloadRequest {
    pub options: DownloadOptions,
    pub queue: Option<crate::QueueId>,
    pub(crate) url: Url,
    pub destination: PathBuf,
    pub replacement: ReplacementPolicy,
    pub deferred: bool,
    pub category: Option<String>,
    pub description: String,
    pub(crate) authorization: Option<Authorization>,
    /// Set only after the caller explicitly chooses an additional copy.
    pub allow_duplicate: bool,
    pub(crate) recovery: Option<crate::storage::Recovery>,
}
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Authorization {
    pub login: String,
    pub password: String,
}
impl DownloadRequest {
    pub fn same_source(&self, other: &Self) -> bool {
        let mut a = self.url.clone();
        let mut b = other.url.clone();
        a.set_fragment(None);
        b.set_fragment(None);
        a == b && self.authorization == other.authorization
    }
    pub fn source_host(&self) -> &str {
        self.url.host_str().unwrap_or("")
    }
    pub fn display_url(&self) -> String {
        let mut url = self.url.clone();
        let _ = url.set_username("");
        let _ = url.set_password(None);
        url.set_query(None);
        url.set_fragment(None);
        url.into()
    }
    pub fn with_authorization(mut self, login: String, password: String) -> Self {
        self.authorization = Some(Authorization { login, password });
        self
    }
}
impl fmt::Debug for DownloadRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DownloadRequest")
            .field("source_host", &self.source_host())
            .field("destination", &self.destination)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Debug)]
pub struct JobSnapshot {
    pub id: JobId,
    pub entry_id: Option<crate::EntryId>,
    pub restored: bool,
    pub recovery_paths: Vec<PathBuf>,
    pub queue_id: Option<crate::QueueId>,
    pub readiness: crate::EntryReadiness,
    pub filename: String,
    pub source_host: String,
    pub display_url: String,
    pub destination: PathBuf,
    pub state: JobState,
    pub received: u64,
    pub total: Option<u64>,
    pub resume_capable: bool,
    /// Per-download bytes/sec; None means unlimited.
    pub speed_limit: Option<u64>,
    pub bytes_per_second: u64,
    pub eta_seconds: Option<u64>,
    pub last_try: Option<SystemTime>,
    pub error: Option<JobError>,
    pub deferred: bool,
    pub category: Option<String>,
    pub description: String,
}
#[derive(Clone, Debug)]
pub struct JobError {
    pub kind: ErrorKind,
    pub message: String,
    pub retained_partial: Option<PathBuf>,
    pub retained_original: Option<PathBuf>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorKind {
    InvalidRequest,
    Http,
    Network,
    LengthMismatch,
    DestinationConflict,
    DestinationChanged,
    Io,
    Cleanup,
    ShuttingDown,
    RestartRequired,
    Storage,
}
impl JobError {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            retained_partial: None,
            retained_original: None,
        }
    }
    pub fn retained(mut self, path: PathBuf) -> Self {
        self.retained_partial = Some(path);
        self
    }
    pub fn original_retained(mut self, path: PathBuf) -> Self {
        self.retained_original = Some(path);
        self
    }
}
impl fmt::Display for JobError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for JobError {}

#[derive(Clone, Copy, Debug)]
pub enum StateAction {
    Submit,
    Admit,
    Cancel,
    Pause,
    PauseFinished,
    Resume,
    TransferSucceeded,
    TransferFailed,
    CancellationFinished,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TransitionError;
impl fmt::Display for TransitionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Invalid job state transition")
    }
}
impl std::error::Error for TransitionError {}

pub fn transition(state: JobState, action: StateAction) -> Result<JobState, TransitionError> {
    use JobState::*;
    use StateAction::*;
    match (state, action) {
        (Waiting, Submit) => Ok(Waiting),
        (Waiting, Admit) => Ok(Running),
        (Waiting, Cancel) => Ok(Cancelled),
        (Running, Cancel) => Ok(Cancelling),
        (Running, Pause) => Ok(Pausing),
        (Pausing, PauseFinished) => Ok(Paused),
        (Paused, Resume) => Ok(Waiting),
        (Pausing | Paused, Cancel) => Ok(Cancelling),
        (Pausing, TransferSucceeded) => Ok(Completed),
        (Pausing | Paused, TransferFailed) => Ok(Failed),
        (Running, TransferSucceeded) => Ok(Completed),
        (Running, TransferFailed) => Ok(Failed),
        (Cancelling, CancellationFinished) => Ok(Cancelled),
        _ => Err(TransitionError),
    }
}

pub fn validate_request(
    url: &str,
    destination: PathBuf,
    replacement: ReplacementPolicy,
) -> Result<DownloadRequest, JobError> {
    let invalid = || {
        JobError::new(
            ErrorKind::InvalidRequest,
            "Enter an HTTP/HTTPS URL and a valid absolute Windows filename.",
        )
    };
    let url = Url::parse(url.trim()).map_err(|_| invalid())?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(invalid());
    }
    // Examine components with Windows separators even in portable model tests.
    let text = destination.to_str().ok_or_else(invalid)?;
    if text.ends_with(['/', '\\']) || text.is_empty() {
        return Err(invalid());
    }
    let normalized = if text.starts_with(r"\\?\") {
        if text.contains('/') {
            return Err(invalid());
        }
        let rest = &text[4..];
        if rest
            .get(..4)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("UNC\\"))
        {
            format!(r"\\{}", &rest[4..])
        } else if rest.as_bytes().get(1) == Some(&b':')
            && rest.as_bytes().get(2) == Some(&b'\\')
            && rest.as_bytes()[0].is_ascii_alphabetic()
        {
            rest.to_owned()
        } else {
            return Err(invalid());
        }
    } else {
        if text.starts_with(r"\\.\") {
            return Err(invalid());
        }
        text.replace('/', "\\")
    };
    let body = if let Some(unc) = normalized.strip_prefix(r"\\") {
        if unc.split('\\').count() < 3 || unc.split('\\').take(2).any(str::is_empty) {
            return Err(invalid());
        }
        unc
    } else if normalized.as_bytes().get(1) == Some(&b':') {
        if !normalized.as_bytes()[0].is_ascii_alphabetic()
            || normalized.as_bytes().get(2) != Some(&b'\\')
        {
            return Err(invalid());
        }
        &normalized[3..]
    } else {
        normalized.as_str()
    };
    for component in body.split('\\') {
        if component.is_empty() {
            continue;
        }
        if component == "."
            || component == ".."
            || component.ends_with(['.', ' '])
            || component.chars().any(|c| c < ' ' || "<>:\"|?*".contains(c))
        {
            return Err(invalid());
        }
        let stem = component
            .split('.')
            .next()
            .unwrap_or("")
            .trim_end()
            .to_uppercase();
        let device = matches!(
            stem.as_str(),
            "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
        ) || ["COM", "LPT"].iter().any(|prefix| {
            stem.strip_prefix(prefix).is_some_and(|suffix| {
                matches!(
                    suffix,
                    "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
                )
            })
        });
        if device {
            return Err(invalid());
        }
    }
    if destination.file_name().is_none() {
        return Err(invalid());
    }
    Ok(DownloadRequest {
        options: Default::default(),
        queue: None,
        url,
        destination,
        replacement,
        deferred: false,
        category: None,
        description: String::new(),
        authorization: None,
        allow_duplicate: false,
        recovery: None,
    })
}

#[cfg(test)]
#[path = "model_tests.rs"]
mod model_tests;

#[derive(Clone, Debug, Default)]
pub struct CategoryRenameUndo {
    pub jobs: Vec<(JobId, Option<String>, Option<String>)>,
    pub entries: Vec<(crate::EntryId, Option<String>, Option<String>)>,
}
