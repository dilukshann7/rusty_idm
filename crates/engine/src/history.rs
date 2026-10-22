//! Durable metadata for the latest attempt of each stable download entry.
use crate::{ErrorKind, JobError, JobSnapshot, JobState};
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, time::SystemTime};

pub(crate) const MAX_HISTORY_ENTRIES: usize = 10_000;
pub(crate) const MAX_STORAGE_BYTES: usize = 16 * 1024 * 1024;
pub(crate) const HISTORY_FULL: &str =
    "Download history is full. Delete history entries or use Delete Completed, then try again.";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum HistoryOutcome {
    Completed,
    Failed,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct HistoryMetadata {
    pub created_at: Option<SystemTime>,
    pub last_try: Option<SystemTime>,
    pub finished_at: Option<SystemTime>,
    pub outcome: Option<HistoryOutcome>,
    pub received: u64,
    pub total: Option<u64>,
    pub error: Option<HistoryFailure>,
}
impl HistoryMetadata {
    pub fn from_snapshot(job: &JobSnapshot) -> Self {
        Self {
            created_at: job.created_at,
            last_try: job.last_try,
            finished_at: job.finished_at,
            outcome: match job.state {
                JobState::Completed => Some(HistoryOutcome::Completed),
                JobState::Failed => Some(HistoryOutcome::Failed),
                _ => None,
            },
            received: job.received,
            total: job.total,
            error: job.error.as_ref().map(HistoryFailure::from_error),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct HistoryFailure {
    kind: ErrorKind,
    message: String,
    retained_partial: Option<PathBuf>,
    retained_original: Option<PathBuf>,
}
impl HistoryFailure {
    pub fn from_error(error: &JobError) -> Self {
        // Persist an allowlisted explanation, never arbitrary server/tool text,
        // URLs or request headers embedded in an in-memory error message.
        let message = match error.kind {
            ErrorKind::InvalidRequest => "The download request was invalid.",
            ErrorKind::Http => "The server rejected the download request.",
            ErrorKind::Network => "The network request failed.",
            ErrorKind::LengthMismatch => "The received file size did not match the expected size.",
            ErrorKind::DestinationConflict => {
                "The destination already exists; choose a filename or approve replacement again."
            }
            ErrorKind::DestinationChanged => {
                "The destination changed; replacement needs a new approval."
            }
            ErrorKind::Io => "A file operation failed.",
            ErrorKind::Cleanup => {
                "Recovery files require attention before this download can continue."
            }
            ErrorKind::ShuttingDown => "The download engine was shutting down.",
            ErrorKind::RestartRequired => {
                "The saved download requires restart or a fresh browser capture."
            }
            ErrorKind::Storage => {
                "Download storage could not be saved; the previous state was preserved."
            }
        };
        Self {
            kind: error.kind,
            message: message.into(),
            retained_partial: error.retained_partial.clone(),
            retained_original: error.retained_original.clone(),
        }
    }
    pub fn to_error(&self) -> JobError {
        JobError {
            kind: self.kind,
            retryable: false,
            http_status: None,
            message: self.message.clone(),
            retained_partial: self.retained_partial.clone(),
            retained_original: self.retained_original.clone(),
        }
    }
}

#[cfg(test)]
#[path = "history_tests.rs"]
mod tests;
