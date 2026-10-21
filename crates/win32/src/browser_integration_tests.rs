use super::*;
use rusty_browser_bridge::protocol::CaptureSource;
fn capture() -> CaptureRequest {
    CaptureRequest {
        browser: Default::default(),
        forced: false,
        policy_revision: 0,
        original_url: None,
        source_page: None,
        request_context: vec![],
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
    let engine = rusty_engine::spawn_engine_in_memory(Arc::new(|| {}));
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

#[test]
fn actor_receipt_and_abort_are_served_while_another_modal_is_open() {
    let engine = rusty_engine::spawn_engine_in_memory(Arc::new(|| {}));
    let controller = BrowserController::new(engine.client(), Arc::new(|| {}));
    let router = controller.router();
    let request = capture();
    let id = request.id.clone();
    let (tx, rx) = mpsc::sync_channel(4);
    router.dispatch(Operation::Capture(request), tx).unwrap();
    assert_eq!(rx.recv().unwrap().result, ReplyStatus::Pending);
    let incoming = controller.drain().pop().unwrap();
    let mut held = rusty_engine::validate_request(
        "https://example.org/file",
        std::path::PathBuf::from("C:/Downloads/held.bin"),
        rusty_engine::ReplacementPolicy::Deny,
    )
    .unwrap();
    held.deferred = true;
    controller
        .submit(
            id.clone(),
            SubmissionAction::Submit(held),
            incoming.lease,
            incoming.reply,
        )
        .unwrap();
    let accepted = rx.recv_timeout(Duration::from_secs(3)).unwrap().result;
    assert!(matches!(
        accepted,
        ReplyStatus::Accepted {
            created_new: true,
            ..
        }
    ));
    assert_eq!(
        controller.take_presentations(),
        vec![rusty_engine::JobId(1)]
    );
    let mut second = capture();
    second.id = RequestId::parse("12345678-1234-4234-8234-123456789abd").unwrap();
    let (pending_tx, pending_rx) = mpsc::sync_channel(4);
    router
        .dispatch(Operation::Capture(second), pending_tx)
        .unwrap();
    assert_eq!(pending_rx.recv().unwrap().result, ReplyStatus::Pending);
    // Do not drain the second capture: model a GUI occupied by its modal dialog.
    let (tx, rx) = mpsc::sync_channel(4);
    router
        .dispatch(
            Operation::Status {
                capture_id: id.clone(),
            },
            tx,
        )
        .unwrap();
    assert_eq!(
        rx.recv_timeout(Duration::from_secs(3)).unwrap().result,
        accepted
    );
    let (tx, rx) = mpsc::sync_channel(4);
    router
        .dispatch(
            Operation::Abort {
                capture_id: id.clone(),
            },
            tx,
        )
        .unwrap();
    assert_eq!(
        rx.recv_timeout(Duration::from_secs(3)).unwrap().result,
        ReplyStatus::Aborted
    );
    let (tx, rx) = mpsc::sync_channel(4);
    router
        .dispatch(
            Operation::Status {
                capture_id: id.clone(),
            },
            tx,
        )
        .unwrap();
    assert_eq!(
        rx.recv_timeout(Duration::from_secs(3)).unwrap().result,
        ReplyStatus::Declined {
            code: DeclineCode::BrowserFallback
        }
    );
    let (tx, rx) = mpsc::sync_channel(4);
    router.dispatch(Operation::Capture(capture()), tx).unwrap();
    assert_eq!(
        rx.recv_timeout(Duration::from_secs(3)).unwrap().result,
        accepted
    );
    drop(router);
    drop(controller);
    engine.try_send(rusty_engine::Command::Shutdown).unwrap();
    engine.join().unwrap();
}

#[test]
fn thirty_third_pending_capture_declines_busy() {
    let engine = rusty_engine::spawn_engine_in_memory(Arc::new(|| {}));
    let controller = BrowserController::new(engine.client(), Arc::new(|| {}));
    let router = controller.router();
    let mut receivers = Vec::new();
    for index in 0..33 {
        let mut request = capture();
        request.id = RequestId::parse(&format!("12345678-1234-4234-8234-{index:012x}")).unwrap();
        let (tx, rx) = mpsc::sync_channel(4);
        router.dispatch(Operation::Capture(request), tx).unwrap();
        assert_eq!(
            rx.recv().unwrap().result,
            if index < 32 {
                ReplyStatus::Pending
            } else {
                ReplyStatus::Declined {
                    code: DeclineCode::Busy,
                }
            }
        );
        receivers.push(rx);
    }
    assert_eq!(controller.drain().len(), 32);
    drop(receivers);
    drop(router);
    drop(controller);
    engine.try_send(rusty_engine::Command::Shutdown).unwrap();
    engine.join().unwrap();
}
