use super::*;
use rusty_browser_bridge::protocol::CaptureSource;
fn capture() -> CaptureRequest {
    CaptureRequest {
        id: RequestId::parse("12345678-1234-4234-8234-123456789abc").unwrap(),
        url: "https://example.org/file".into(),
        filename_hint: None,
        source: CaptureSource::ContextMenu,
        download_id: None,
        expires_at_ms: u64::MAX,
    }
}
#[test]
fn same_capture_id_shows_one_dialog_and_closing_revokes_pending() {
    let engine = rusty_engine::spawn_engine(Arc::new(|| {}));
    let controller = BrowserController::new(engine.client(), Arc::new(|| {}));
    let router = controller.router();
    let request = capture();
    let (tx, rx) = mpsc::sync_channel(4);
    router
        .dispatch(Operation::Capture(request.clone()), tx)
        .unwrap();
    assert_eq!(rx.recv().unwrap().result, ReplyStatus::Pending);
    let (tx, _rx) = mpsc::sync_channel(4);
    router.dispatch(Operation::Capture(request), tx).unwrap();
    let inbox = controller.drain();
    assert_eq!(inbox.len(), 1);
    controller.close();
    assert_ne!(inbox[0].lease.state(), LeaseState::Pending);
    drop(router);
    drop(controller);
    engine.try_send(rusty_engine::Command::Shutdown).unwrap();
    engine.join().unwrap();
}
