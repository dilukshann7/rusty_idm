use crate::{DownloadRequest, JobId};
use rusty_browser_bridge::{lease::CaptureLease, protocol::RequestId};
use tokio::sync::oneshot;
#[derive(Clone, Debug)]
pub enum SubmissionAction {
    Submit(DownloadRequest),
    Show(JobId),
    Activate(JobId),
    Restart(JobId),
    RestartApproved(JobId, crate::ReplacementPolicy),
}
impl SubmissionAction {
    pub(crate) fn same(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::RestartApproved(a, pa), Self::RestartApproved(b, pb)) => a == b && pa == pb,
            (Self::Show(a), Self::Show(b))
            | (Self::Activate(a), Self::Activate(b))
            | (Self::Restart(a), Self::Restart(b)) => a == b,
            (Self::Submit(a), Self::Submit(b)) => {
                a.same_source(b)
                    && a.options == b.options
                    && a.queue == b.queue
                    && a.destination == b.destination
                    && a.deferred == b.deferred
                    && a.category == b.category
                    && a.description == b.description
                    && a.allow_duplicate == b.allow_duplicate
                    && match (&a.replacement, &b.replacement) {
                        (crate::ReplacementPolicy::Deny, crate::ReplacementPolicy::Deny) => true,
                        (
                            crate::ReplacementPolicy::Approved(a),
                            crate::ReplacementPolicy::Approved(b),
                        ) => a == b,
                        _ => false,
                    }
            }
            _ => false,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubmissionReceipt {
    pub id: RequestId,
    pub job_id: JobId,
    pub created_new: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SubmissionStatus {
    Pending,
    Accepted(SubmissionReceipt),
    Declined(SubmissionError),
    Unknown,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubmissionError {
    Expired,
    Revoked,
    Busy,
    Closing,
    NotFound,
    InvalidAction,
    RecoveryRequired,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CancelOwnedResult {
    Cancelled,
    NotOwned,
    AlreadyFinished,
    Unknown,
}
pub enum ExternalCommand {
    Apply {
        id: RequestId,
        action: SubmissionAction,
        lease: CaptureLease,
        reply: oneshot::Sender<Result<SubmissionReceipt, SubmissionError>>,
    },
    Status {
        id: RequestId,
        reply: oneshot::Sender<SubmissionStatus>,
    },
    CancelOwned {
        id: RequestId,
        reply: oneshot::Sender<CancelOwnedResult>,
    },
}
pub(crate) struct Record {
    pub action: SubmissionAction,
    pub result: Result<SubmissionReceipt, SubmissionError>,
    pub cancelled: bool,
    pub at: std::time::Instant,
}
