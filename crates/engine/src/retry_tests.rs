use crate::{DownloadOptions, ErrorKind, JobError, RetryPolicy};

#[test]
fn policy_defaults_bounds_and_legacy_download_options() {
    let defaults = RetryPolicy::default();
    assert_eq!(defaults.max_retries, 3);
    assert_eq!(defaults.delay_seconds, 5);
    for count in [0, 100] {
        for delay in [1, 3600] {
            assert!(RetryPolicy {
                max_retries: count,
                delay_seconds: delay
            }
            .validate()
            .is_ok());
        }
    }
    assert!(RetryPolicy {
        max_retries: 101,
        ..defaults
    }
    .validate()
    .is_err());
    for delay_seconds in [0, 3601] {
        assert!(RetryPolicy {
            delay_seconds,
            ..defaults
        }
        .validate()
        .is_err());
    }
    let mut saved = serde_json::to_value(DownloadOptions::default()).unwrap();
    saved.as_object_mut().unwrap().remove("retry_policy");
    let old: DownloadOptions = serde_json::from_value(saved).unwrap();
    assert_eq!(old.retry_policy, defaults);
    let saved = serde_json::to_string(&old).unwrap();
    assert_eq!(
        serde_json::from_str::<DownloadOptions>(&saved).unwrap(),
        old
    );
}

#[test]
fn only_explicitly_transient_errors_are_retryable() {
    for kind in [
        ErrorKind::Network,
        ErrorKind::Http,
        ErrorKind::LengthMismatch,
        ErrorKind::Io,
        ErrorKind::Storage,
        ErrorKind::RestartRequired,
    ] {
        assert!(!JobError::new(kind, "failure").retryable);
    }
    assert!(JobError::transient(ErrorKind::Network, "interrupted").retryable);
    assert!(JobError::transient(ErrorKind::LengthMismatch, "short body").retryable);
    for status in [408, 429, 500, 502, 503, 504] {
        let error = JobError::http(status, "server failure");
        assert!(error.retryable, "HTTP {status}");
        assert_eq!(error.http_status, Some(status));
    }
    for status in [200, 206, 301, 400, 401, 403, 404, 416, 501, 505] {
        assert!(
            !JobError::http(status, "response").retryable,
            "HTTP {status}"
        );
    }
}
