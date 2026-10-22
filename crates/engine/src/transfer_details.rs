//! Session-only data-transfer workers. These are logical requests, not TCP sockets.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SegmentState {
    Waiting,
    Connecting,
    Receiving,
    Paused,
    Completed,
    Failed,
    Cancelled,
}
impl SegmentState {
    pub fn is_active(self) -> bool {
        matches!(self, Self::Connecting | Self::Receiving)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SegmentSnapshot {
    pub id: u32,
    /// Inclusive byte offsets; an unknown response length has no end offset.
    pub start: u64,
    pub end: Option<u64>,
    pub received: u64,
    pub state: SegmentState,
    pub bytes_per_second: u64,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransferDetails {
    pub connection_limit: u8,
    pub active_connections: usize,
    pub segments: Vec<SegmentSnapshot>,
}

/// Raw worker counters, coalesced with aggregate progress before actor sampling.
#[derive(Clone, Debug)]
pub struct SegmentProgress {
    pub id: u32,
    pub start: u64,
    pub end: Option<u64>,
    pub received: u64,
    /// Accepted network bytes in this connection generation; excludes saved bytes.
    pub transferred: u64,
    /// Changes when a paused worker is reconnected, resetting its speed baseline.
    pub generation: u64,
    pub state: SegmentState,
}
#[derive(Clone, Debug)]
pub struct ConnectionProgress {
    pub connection_limit: u8,
    pub segments: Vec<SegmentProgress>,
}
