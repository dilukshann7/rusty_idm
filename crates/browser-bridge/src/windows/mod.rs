mod identity;
mod pipe;
use crate::{
    lease::CaptureLease,
    protocol::{CaptureRequest, Operation, Reply},
};
pub use identity::{OwnedHandle, SecurityDescriptor, UserSession};
pub use pipe::{PipeClient, PipeServer, ReplyStream};
use std::{io, sync::mpsc::SyncSender};
pub type BridgeError = io::Error;
pub trait RequestRouter: Send + Sync {
    fn dispatch(&self, operation: Operation, replies: SyncSender<Reply>)
        -> Result<(), BridgeError>;
}
pub struct IncomingCapture {
    pub capture: CaptureRequest,
    pub lease: CaptureLease,
    pub reply: SyncSender<Reply>,
}
