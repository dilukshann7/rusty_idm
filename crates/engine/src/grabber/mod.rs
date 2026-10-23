//! Bounded public-site discovery, durable projects, and offline publication.
mod crawl;
mod document;
mod export;
mod model;
mod runtime;
mod store;

pub(crate) use crawl::redirect_policy;
pub(crate) use model::host_allowed;
pub use model::*;
pub(crate) use runtime::{spawn, Handle};
#[doc(hidden)]
pub use runtime::{TransferAction, TransferStatus};

/// Commands are acknowledged independently of coalesced progress notifications.
pub struct GrabberCommand {
    pub action: GrabberAction,
    pub reply: std::sync::mpsc::SyncSender<Result<GrabberReply, String>>,
}

pub enum GrabberAction {
    Query {
        project: Option<ProjectId>,
        offset: usize,
        filter: String,
    },
    Save {
        project: Option<ProjectId>,
        config: ProjectConfig,
        rescan: bool,
    },
    Scan(ProjectId),
    Stop(ProjectId),
    Resume(ProjectId),
    Download(ProjectId),
    RetryFailed(ProjectId),
    Select {
        project: ProjectId,
        items: Vec<ItemId>,
        selected: bool,
    },
    Delete(ProjectId),
}

#[derive(Clone, Debug)]
pub struct GrabberEvent {
    pub projects: Vec<ProjectSnapshot>,
    pub error: Option<String>,
}

#[derive(Clone, Debug)]
pub struct GrabberReply {
    pub projects: Vec<ProjectSnapshot>,
    pub project: Option<ProjectSnapshot>,
    pub config: Option<ProjectConfig>,
    pub results: Vec<GrabberItem>,
    pub offset: usize,
    pub total: usize,
}
