use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct QueueId(pub u64);
impl QueueId {
    pub const MAIN: Self = Self(1);
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct EntryId(pub u64);
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueueSnapshot {
    pub id: QueueId,
    pub name: String,
    pub parallelism: u8,
    pub revision: u64,
    pub schedule: crate::QueueSchedule,
    pub schedule_status: crate::ScheduleStatus,
    pub running: bool,
    pub stopping: bool,
    pub entries: Vec<EntryId>,
    pub completed: usize,
    pub active: usize,
    pub error: Option<String>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryReadiness {
    Ready,
    CredentialsRequired,
    RestartRequired,
    RecoveryRequired,
}
