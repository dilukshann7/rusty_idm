mod support;
use rusty_engine::{
    transfer::{transfer, TransferOutcome},
    validate_request, ReplacementPolicy,
};
use std::sync::{Arc, Mutex};
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn exact_empty_unknown_and_redirect_bodies() {
    let fixture = support::Fixture::new(support::respond);
    for path in ["/data", "/empty", "/unknown", "/redirect"] {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("file.bin");
        let request = validate_request(
            &format!("{}{path}", fixture.url),
            target.clone(),
            ReplacementPolicy::Deny,
        )
        .unwrap();
        let progress = Arc::new(Mutex::new(Vec::new()));
        let seen = progress.clone();
        let result = transfer(
            request,
            CancellationToken::new(),
            Arc::new(move |p| seen.lock().unwrap().push(p)),
        )
        .await
        .unwrap();
        let expected = if path == "/empty" {
            vec![]
        } else {
            support::bytes()
        };
        assert_eq!(
            result,
            TransferOutcome::Completed {
                bytes: expected.len() as u64
            }
        );
        assert_eq!(std::fs::read(&target).unwrap(), expected);
        if path == "/unknown" {
            assert!(progress.lock().unwrap().iter().all(|p| p.total.is_none()));
        }
    }
}

#[tokio::test]
async fn http_failure_truncation_encoding_and_redirect_limit_leave_no_output() {
    let fixture = support::Fixture::new(support::respond);
    for path in ["/404", "/503", "/broken", "/encoded", "/loop", "/partial"] {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("file.bin");
        let request = validate_request(
            &format!("{}{path}", fixture.url),
            target.clone(),
            ReplacementPolicy::Deny,
        )
        .unwrap();
        assert!(
            transfer(request, CancellationToken::new(), Arc::new(|_| {}))
                .await
                .is_err(),
            "{path}"
        );
        assert!(!target.exists());
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    }
}

#[tokio::test]
async fn cancellation_cleans_partial() {
    let fixture = support::Fixture::new(support::respond);
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("file.bin");
    let request = validate_request(
        &format!("{}/slow", fixture.url),
        target.clone(),
        ReplacementPolicy::Deny,
    )
    .unwrap();
    let cancel = CancellationToken::new();
    let stop = cancel.clone();
    let result = transfer(
        request,
        cancel,
        Arc::new(move |p| {
            if p.received > 0 {
                stop.cancel();
            }
        }),
    )
    .await
    .unwrap();
    assert_eq!(result, TransferOutcome::Cancelled);
    assert!(!target.exists());
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
}

#[tokio::test]
async fn authorization_is_applied_only_when_explicitly_supplied() {
    let fixture = support::Fixture::new(support::respond);
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("private.bin");
    let request = validate_request(
        &format!("{}/auth", fixture.url),
        target.clone(),
        ReplacementPolicy::Deny,
    )
    .unwrap();
    assert!(
        transfer(request.clone(), CancellationToken::new(), Arc::new(|_| {}))
            .await
            .is_err()
    );
    assert!(!target.exists());
    let result = transfer(
        request.with_authorization("user".into(), "pass".into()),
        CancellationToken::new(),
        Arc::new(|_| {}),
    )
    .await
    .unwrap();
    assert_eq!(result, TransferOutcome::Completed { bytes: 2 });
    assert_eq!(std::fs::read(target).unwrap(), b"ok");
}
