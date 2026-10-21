use super::*;

#[test]
fn duplicate_matching_ignores_fragment_but_preserves_query_and_authorization() {
    let make = |url| {
        validate_request(
            url,
            PathBuf::from("C:/Downloads/file.bin"),
            ReplacementPolicy::Deny,
        )
        .unwrap()
    };
    let a = make("https://example.com/file?token=one#chapter");
    assert!(a.same_source(&make("https://example.com/file?token=one#different")));
    assert!(!a.same_source(&make("https://example.com/file?token=two")));
    assert!(!a.same_source(&a.clone().with_authorization("user".into(), "secret".into())));
    assert!(!format!("{a:?}").contains("token=one"));
}

#[test]
fn cancel_waiting_is_terminal() {
    assert_eq!(
        transition(JobState::Waiting, StateAction::Cancel).unwrap(),
        JobState::Cancelled
    );
}

#[test]
fn running_cancel_waits_for_cleanup() {
    assert_eq!(
        transition(JobState::Running, StateAction::Cancel).unwrap(),
        JobState::Cancelling
    );
}

#[test]
fn completion_cannot_override_cancellation() {
    assert!(transition(JobState::Cancelling, StateAction::TransferSucceeded).is_err());
}

#[test]
fn pause_requires_acknowledgement_and_resume_returns_to_queue() {
    assert_eq!(
        transition(JobState::Running, StateAction::Pause).unwrap(),
        JobState::Pausing
    );
    assert_eq!(
        transition(JobState::Pausing, StateAction::PauseFinished).unwrap(),
        JobState::Paused
    );
    assert_eq!(
        transition(JobState::Paused, StateAction::Resume).unwrap(),
        JobState::Waiting
    );
    assert_eq!(
        transition(JobState::Paused, StateAction::Cancel).unwrap(),
        JobState::Cancelling
    );
    assert!(!JobState::Paused.is_terminal());
    assert!(!JobState::Paused.can_retry());
    assert!(transition(JobState::Completed, StateAction::Resume).is_err());
}

#[test]
fn validates_windows_names_and_web_urls() {
    for name in [
        "",
        "CON",
        "nul.txt",
        "COM1.zip",
        "LPT9",
        "a:b",
        "a?.txt",
        "a.",
        "a ",
        "..",
        "COM¹.txt",
    ] {
        assert!(
            validate_request(
                "https://example.com/file",
                PathBuf::from(name),
                ReplacementPolicy::Deny
            )
            .is_err(),
            "{name}"
        );
    }
    for url in [
        "",
        "garbage",
        "file:///c:/x",
        "ftp://example.com/a",
        "https://",
    ] {
        assert!(validate_request(
            url,
            PathBuf::from("C:/Downloads/test.bin"),
            ReplacementPolicy::Deny
        )
        .is_err());
    }
    let path = PathBuf::from("C:/Downloads/தமிழ் 日本語.zip");
    assert_eq!(
        validate_request(
            "https://example.com/file?secret=123",
            path.clone(),
            ReplacementPolicy::Deny
        )
        .unwrap()
        .destination,
        path
    );
}

#[test]
fn removal_and_retry_only_for_terminal_jobs() {
    for state in [JobState::Waiting, JobState::Running, JobState::Cancelling] {
        assert!(!state.can_retry());
        assert!(!state.is_terminal());
    }
    assert!(!JobState::Completed.can_retry());
    assert!(JobState::Failed.can_retry());
    assert!(JobState::Cancelled.can_retry());
}

#[test]
fn display_url_does_not_reveal_authorization_queries_or_fragments() {
    let request = validate_request(
        "https://example.com/file?token=secret#secret",
        "file.bin".into(),
        ReplacementPolicy::Deny,
    )
    .unwrap()
    .with_authorization("user".into(), "pass".into());
    assert_eq!(request.source_host(), "example.com");
    assert!(!format!("{request:?}").contains("secret"));
    assert!(!format!("{request:?}").contains("pass"));
}

#[test]
fn accepts_verbatim_drive_and_unc_paths_but_rejects_device_namespaces() {
    for path in [
        r"\\?\C:\Downloads\தமிழ்.bin",
        r"\\?\UNC\server\share\file.bin",
        r"\\server\share\file.bin",
    ] {
        assert!(
            validate_request(
                "https://example.com/file",
                path.into(),
                ReplacementPolicy::Deny
            )
            .is_ok(),
            "{path}"
        );
    }
    for path in [
        r"\\.\PhysicalDrive0",
        r"\\?\GLOBALROOT\Device\HarddiskVolume1\file",
        r"\\?\C:\Downloads\file.bin:stream",
        r"\\?\UNC\server\share",
        r"C:relative.bin",
    ] {
        assert!(
            validate_request(
                "https://example.com/file",
                path.into(),
                ReplacementPolicy::Deny
            )
            .is_err(),
            "{path}"
        );
    }
}

#[test]
fn authorization_is_never_exposed_by_request_debug() {
    let request = validate_request(
        "https://example.com/private",
        "file.bin".into(),
        ReplacementPolicy::Deny,
    )
    .unwrap()
    .with_authorization("private-user".into(), "private-password".into());
    let debug = format!("{request:?}");
    assert!(!debug.contains("private-user"));
    assert!(!debug.contains("private-password"));
}

#[test]
fn status_display_url_keeps_the_path_and_removes_query_and_fragment() {
    let request = validate_request(
        "https://example.com/files/video.mp4?token=private#secret",
        "video.mp4".into(),
        ReplacementPolicy::Deny,
    )
    .unwrap();
    assert_eq!(request.display_url(), "https://example.com/files/video.mp4");
}
