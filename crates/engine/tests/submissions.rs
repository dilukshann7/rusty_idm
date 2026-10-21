use rusty_browser_bridge::{lease::CaptureLease, protocol::RequestId};
use rusty_engine::{submissions::*, *};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
fn id() -> RequestId {
    RequestId::parse("12345678-1234-4234-8234-123456789abc").unwrap()
}
fn apply(
    engine: &EngineHandle,
    action: SubmissionAction,
    lease: CaptureLease,
) -> Result<SubmissionReceipt, SubmissionError> {
    let (reply, rx) = tokio::sync::oneshot::channel();
    engine
        .try_external(ExternalCommand::Apply {
            id: id(),
            action,
            lease,
            reply,
        })
        .unwrap();
    rx.blocking_recv().unwrap()
}
fn lease() -> CaptureLease {
    CaptureLease::new(Instant::now() + Duration::from_secs(10))
}
fn shutdown(engine: EngineHandle) {
    engine.try_send(Command::Shutdown).unwrap();
    engine.join().unwrap();
}
#[test]
fn same_id_returns_same_job_once_and_status_survives_event_drain() {
    let dir = tempfile::tempdir().unwrap();
    let mut request = validate_request(
        "https://example.org/file",
        dir.path().join("file.bin"),
        ReplacementPolicy::Deny,
    )
    .unwrap();
    request.deferred = true;
    let engine = spawn_engine_in_memory(Arc::new(|| {}));
    let first = apply(&engine, SubmissionAction::Submit(request.clone()), lease()).unwrap();
    assert_eq!(
        first,
        apply(&engine, SubmissionAction::Submit(request.clone()), lease()).unwrap()
    );
    assert_eq!(engine.destinations(), vec![request.destination.clone()]);
    engine.drain_events();
    let (reply, rx) = tokio::sync::oneshot::channel();
    engine
        .try_external(ExternalCommand::Status { id: id(), reply })
        .unwrap();
    assert_eq!(
        rx.blocking_recv().unwrap(),
        SubmissionStatus::Accepted(first)
    );
    request.destination = dir.path().join("changed.bin");
    assert_eq!(
        apply(&engine, SubmissionAction::Submit(request), lease()),
        Err(SubmissionError::InvalidAction)
    );
    shutdown(engine);
}
#[test]
fn revoked_or_expired_request_never_admits() {
    let dir = tempfile::tempdir().unwrap();
    let mut request = validate_request(
        "https://example.org/file",
        dir.path().join("never-created.bin"),
        ReplacementPolicy::Deny,
    )
    .unwrap();
    request.deferred = true;
    let revoked = lease();
    revoked.revoke();
    let expired = CaptureLease::new(Instant::now() - Duration::from_secs(1));
    for (lease, error) in [
        (revoked, SubmissionError::Revoked),
        (expired, SubmissionError::Expired),
    ] {
        let engine = spawn_engine_in_memory(Arc::new(|| {}));
        assert_eq!(
            apply(&engine, SubmissionAction::Submit(request.clone()), lease),
            Err(error)
        );
        assert!(engine.destinations().is_empty());
        assert!(!request.destination.exists());
        shutdown(engine);
    }
}
#[test]
fn fallback_cancels_only_new_owned_job() {
    let dir = tempfile::tempdir().unwrap();
    let mut request = validate_request(
        "https://example.org/file",
        dir.path().join("file.bin"),
        ReplacementPolicy::Deny,
    )
    .unwrap();
    request.deferred = true;
    let engine = spawn_engine_in_memory(Arc::new(|| {}));
    let receipt = apply(&engine, SubmissionAction::Submit(request.clone()), lease()).unwrap();
    let (reply, rx) = tokio::sync::oneshot::channel();
    engine
        .try_external(ExternalCommand::CancelOwned { id: id(), reply })
        .unwrap();
    assert_eq!(rx.blocking_recv().unwrap(), CancelOwnedResult::Cancelled);
    assert_eq!(
        apply(&engine, SubmissionAction::Submit(request), lease()).unwrap(),
        receipt
    );
    shutdown(engine);
}

#[test]
fn dropped_reply_receiver_preserves_the_original_receipt() {
    let dir = tempfile::tempdir().unwrap();
    let mut request = validate_request(
        "https://example.org/file",
        dir.path().join("held.bin"),
        ReplacementPolicy::Deny,
    )
    .unwrap();
    request.deferred = true;
    let engine = spawn_engine_in_memory(Arc::new(|| {}));
    let (reply, rx) = tokio::sync::oneshot::channel();
    drop(rx);
    engine
        .try_external(ExternalCommand::Apply {
            id: id(),
            action: SubmissionAction::Submit(request),
            lease: lease(),
            reply,
        })
        .unwrap();
    let (reply, rx) = tokio::sync::oneshot::channel();
    engine
        .try_external(ExternalCommand::Status { id: id(), reply })
        .unwrap();
    assert!(matches!(
        rx.blocking_recv().unwrap(),
        SubmissionStatus::Accepted(SubmissionReceipt {
            job_id: JobId(1),
            created_new: true,
            ..
        })
    ));
    shutdown(engine);
}

#[test]
fn reused_existing_entry_is_never_owned_by_browser_fallback() {
    let dir = tempfile::tempdir().unwrap();
    let mut request = validate_request(
        "https://example.org/file",
        dir.path().join("held.bin"),
        ReplacementPolicy::Deny,
    )
    .unwrap();
    request.deferred = true;
    let engine = spawn_engine_in_memory(Arc::new(|| {}));
    engine.try_send(Command::Submit(request.clone())).unwrap();
    let receipt = apply(&engine, SubmissionAction::Show(JobId(1)), lease()).unwrap();
    assert!(!receipt.created_new);
    let (reply, rx) = tokio::sync::oneshot::channel();
    engine
        .try_external(ExternalCommand::CancelOwned { id: id(), reply })
        .unwrap();
    assert_eq!(rx.blocking_recv().unwrap(), CancelOwnedResult::NotOwned);
    assert_eq!(
        engine.find_duplicate(&request).unwrap().state,
        JobState::Waiting
    );
    shutdown(engine);
}
