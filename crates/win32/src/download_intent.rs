use crate::dialogs::AddedDownload;
use rusty_engine::{submissions::SubmissionAction, JobId};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExistingAction {
    Show,
    Activate,
    Restart,
}
pub enum DownloadIntent {
    New(AddedDownload),
    Existing {
        job_id: JobId,
        action: ExistingAction,
    },
    RestartApproved {
        job_id: JobId,
        replacement: rusty_engine::ReplacementPolicy,
    },
    Cancelled,
}
impl DownloadIntent {
    pub fn into_submission(self) -> Option<SubmissionAction> {
        match self {
            Self::New(download) => Some(SubmissionAction::Submit(download.request)),
            Self::Existing { job_id, action } => Some(match action {
                ExistingAction::Show => SubmissionAction::Show(job_id),
                ExistingAction::Activate => SubmissionAction::Activate(job_id),
                ExistingAction::Restart => SubmissionAction::Restart(job_id),
            }),
            Self::RestartApproved {
                job_id,
                replacement,
            } => Some(SubmissionAction::RestartApproved(job_id, replacement)),
            Self::Cancelled => None,
        }
    }
}
#[cfg(test)]
#[path = "download_intent_tests.rs"]
mod tests;
