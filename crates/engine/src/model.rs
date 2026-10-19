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
#[derive(Clone, Debug, PartialEq, Eq)]
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

#[derive(Clone)]
pub struct DownloadRequest {
    pub(crate) url: Url,
    pub destination: PathBuf,
    pub replacement: ReplacementPolicy,
    pub deferred: bool,
    pub category: Option<String>,
    pub description: String,
    pub(crate) authorization: Option<Authorization>,
}
#[derive(Clone)]
pub(crate) struct Authorization {
    pub login: String,
    pub password: String,
}
impl DownloadRequest {
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
}
impl JobError {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            retained_partial: None,
        }
    }
    pub fn retained(mut self, path: PathBuf) -> Self {
        self.retained_partial = Some(path);
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
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
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
        url,
        destination,
        replacement,
        deferred: false,
        category: None,
        description: String::new(),
        authorization: None,
    })
}

#[cfg(test)]
#[path = "model_tests.rs"]
mod model_tests;
