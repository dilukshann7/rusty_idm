#![cfg(windows)]
use rusty_browser_bridge::{protocol::*, windows::*};
use std::{
    sync::{mpsc::SyncSender, Arc},
    time::Duration,
};
struct Router;
impl RequestRouter for Router {
    fn dispatch(
        &self,
        operation: Operation,
        replies: SyncSender<Reply>,
    ) -> Result<(), BridgeError> {
        let id = match operation {
            Operation::Capture(c) => c.id,
            _ => RequestId::parse("00000000-0000-4000-8000-000000000001").unwrap(),
        };
        replies
            .try_send(Reply::new(
                id,
                ReplyStatus::Ok {
                    host_version: "test".into(),
                    gui_running: true,
                },
            ))
            .unwrap();
        Ok(())
    }
}
#[test]
fn occupied_endpoint_fails_closed_and_ping_is_framed() {
    let session = UserSession::current().unwrap();
    let server = PipeServer::start(session.clone(), Arc::new(Router)).unwrap();
    assert!(PipeServer::start(session.clone(), Arc::new(Router)).is_err());
    let mut client = PipeClient::connect(&session, Duration::from_secs(1)).unwrap();
    assert!(matches!(
        client
            .request(Operation::Ping)
            .unwrap()
            .recv()
            .unwrap()
            .result,
        ReplyStatus::Ok {
            gui_running: true,
            ..
        }
    ));
    drop(server);
}
#[test]
fn other_logon_and_user_cannot_match_identity() {
    let session = UserSession::current().unwrap();
    let mut other = session.clone();
    other.session_id = other.session_id.wrapping_add(1);
    assert!(!session.same_identity(&other));
    other = session.clone();
    other.logon_sid.push_str("-1");
    assert!(!session.same_identity(&other));
}
