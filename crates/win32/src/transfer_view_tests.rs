use super::*;

#[test]
fn connection_summary_and_segment_rows_use_live_telemetry() {
    use rusty_engine::{SegmentSnapshot, SegmentState, TransferDetails};
    let mut snapshot = job();
    assert!(format_transfer(&snapshot)
        .details
        .contains("Connections: Unavailable"));
    snapshot.transfer_details = Some(TransferDetails {
        connection_limit: 8,
        active_connections: 2,
        segments: vec![SegmentSnapshot {
            id: 7,
            start: 100,
            end: Some(199),
            received: 25,
            state: SegmentState::Receiving,
            bytes_per_second: 1024,
        }],
    });
    assert!(format_transfer(&snapshot)
        .details
        .contains("Connections: 2 active (limit 8)"));
    let rows = format_segments(&snapshot);
    assert_eq!(
        rows[0].cells,
        ["8", "100–199", "25.00 %", "Receiving", "1.000 KB/sec"]
    );
    assert_eq!(rows[0].progress, Some(2500));
    snapshot.state = JobState::Paused;
    assert_eq!(format_segments(&snapshot)[0].cells[4], "0 Bytes/sec");
}

#[test]
fn segment_percentages_handle_unknown_empty_and_full_u64_ranges() {
    use rusty_engine::{SegmentSnapshot, SegmentState, TransferDetails};
    let mut snapshot = job();
    snapshot.transfer_details = Some(TransferDetails {
        connection_limit: 1,
        active_connections: 1,
        segments: vec![SegmentSnapshot {
            id: 0,
            start: 0,
            end: None,
            received: 1,
            state: SegmentState::Receiving,
            bytes_per_second: 0,
        }],
    });
    assert_eq!(format_segments(&snapshot)[0].progress, None);
    snapshot.transfer_details.as_mut().unwrap().segments[0].end = Some(u64::MAX);
    snapshot.transfer_details.as_mut().unwrap().segments[0].received = u64::MAX;
    assert_eq!(format_segments(&snapshot)[0].progress, Some(9999));
    snapshot.state = JobState::Completed;
    assert_eq!(format_segments(&snapshot)[0].progress, Some(10_000));
    snapshot.total = Some(0);
    let row = &mut snapshot.transfer_details.as_mut().unwrap().segments[0];
    row.end = None;
    row.received = 0;
    assert_eq!(format_segments(&snapshot)[0].cells[1], "Empty");
    assert_eq!(format_segments(&snapshot)[0].progress, Some(10_000));
}

#[test]
fn segment_table_sync_preserves_rows_and_only_changes_modified_cells() {
    use rusty_engine::{SegmentSnapshot, SegmentState, TransferDetails};
    let mut snapshot = job();
    snapshot.transfer_details = Some(TransferDetails {
        connection_limit: 16,
        active_connections: 0,
        segments: (0..128)
            .map(|id| SegmentSnapshot {
                id,
                start: u64::from(id) * 100,
                end: Some(u64::from(id) * 100 + 99),
                received: 0,
                state: SegmentState::Waiting,
                bytes_per_second: 0,
            })
            .collect(),
    });
    let mut table = SegmentTableRows::default();
    assert_eq!(table.sync(format_segments(&snapshot)).len(), 128 * 6);
    assert!(table.sync(format_segments(&snapshot)).is_empty());
    snapshot.transfer_details.as_mut().unwrap().segments[64].state = SegmentState::Connecting;
    assert_eq!(
        table.sync(format_segments(&snapshot)),
        vec![SegmentChange::SetCell { row: 64, column: 3 }]
    );
    snapshot
        .transfer_details
        .as_mut()
        .unwrap()
        .segments
        .remove(0);
    assert_eq!(
        table.sync(format_segments(&snapshot)),
        vec![SegmentChange::Remove(0)]
    );
    assert_eq!(table.rows[0].id, 1);
}

#[test]
fn restored_history_does_not_open_progress_windows_and_unknown_dates_are_explicit() {
    let mut snapshot = job();
    snapshot.state = JobState::Failed;
    snapshot.restored = true;
    let mut tracker = AutoOpenTracker::default();
    assert!(!tracker.should_open(&snapshot));
    snapshot.state = JobState::Completed;
    assert!(!tracker.should_open(&snapshot));
    let text = format_transfer(&snapshot);
    assert!(text.details.contains("Added: Unknown"));
    assert!(text.details.contains("Finished: Unknown"));
    snapshot.restored = false;
    snapshot.state = JobState::Running;
    snapshot.last_try = Some(SystemTime::UNIX_EPOCH + Duration::from_secs(2));
    assert!(tracker.should_open(&snapshot));
}

#[test]
fn paused_status_preserves_percentage_and_reports_resume_capability() {
    let mut snapshot = job();
    snapshot.state = JobState::Paused;
    snapshot.resume_capable = true;
    snapshot.speed_limit = Some(65_536);
    let text = format_transfer(&snapshot);
    assert_eq!(text.status, "Paused.");
    assert!(text.details.contains("Resume capability: Yes"));
    assert!(text.details.contains("64.000 KB/sec"));
    assert_eq!(text.rate, "0 Bytes/sec");
}
use rusty_engine::{ErrorKind, JobError};
use std::{
    path::PathBuf,
    time::{Duration, SystemTime},
};

fn job() -> JobSnapshot {
    JobSnapshot {
        entry_id: None,
        restored: false,
        recovery_paths: vec![],
        queue_id: None,
        readiness: rusty_engine::EntryReadiness::Ready,
        id: JobId(1),
        filename: "sample.mp4".into(),
        source_host: "example.com".into(),
        display_url: "https://example.com/sample.mp4".into(),
        destination: PathBuf::from("C:/Downloads/sample.mp4"),
        state: JobState::Running,
        received: 8354,
        total: Some(10000),
        resume_capable: false,
        refreshable: false,
        speed_limit: None,
        bytes_per_second: 1_048_576,
        transfer_details: None,
        eta_seconds: Some(0),
        created_at: None,
        finished_at: None,
        last_try: Some(SystemTime::UNIX_EPOCH + Duration::from_secs(1)),
        retry: None,
        error: None,
        deferred: false,
        category: None,
        description: String::new(),
    }
}
#[test]
fn status_fields_match_the_reference_precision_without_overflow() {
    let view = format_transfer(&job());
    assert_eq!(view.title, "83% sample.mp4");
    assert!(view.downloaded.ends_with("( 83.54 % )"));
    assert_eq!(view.rate, "1.000 MB/sec");
    assert_eq!(view.time_left, "0 sec");
    let mut big = job();
    big.received = u64::MAX;
    big.total = Some(u64::MAX);
    assert!(format_transfer(&big).downloaded.ends_with("( 100.00 % )"));
}
#[test]
fn pending_recovery_shows_attempt_and_last_error_without_claiming_terminal_failure() {
    let mut snapshot = job();
    snapshot.state = JobState::Waiting;
    snapshot.retry = Some(rusty_engine::RetryStatus {
        attempt: 1,
        max_retries: 3,
        next_retry_at: None,
    });
    snapshot.error = Some(rusty_engine::JobError::transient(
        rusty_engine::ErrorKind::Network,
        "Connection interrupted",
    ));
    let view = format_transfer(&snapshot);
    assert_eq!(view.status, "Retry 1/3 paused");
    assert!(view.details.contains("Connection interrupted"));
    assert_eq!(view.rate, "0 Bytes/sec");
    assert_eq!(pause_action(&snapshot), Some(false));
    snapshot.retry.as_mut().unwrap().next_retry_at = Some(SystemTime::now());
    assert_eq!(pause_action(&snapshot), Some(true));
    snapshot.retry = None;
    assert_eq!(pause_action(&snapshot), None);
}
#[test]
fn unknown_length_does_not_invent_percentage_or_eta_and_empty_completion_is_truthful() {
    let mut job = job();
    job.total = None;
    job.eta_seconds = Some(0);
    let view = format_transfer(&job);
    assert_eq!(view.title, "sample.mp4");
    assert!(!view.downloaded.contains('%'));
    assert_eq!(view.time_left, "Unknown");
    job.total = Some(0);
    job.received = 0;
    job.state = JobState::Completed;
    let view = format_transfer(&job);
    assert_eq!(view.title, "100% sample.mp4");
    assert_eq!(view.size, "0 Bytes");
}
#[test]
fn auto_open_occurs_once_per_attempt_and_allows_retry_without_reopening_after_manual_close() {
    let mut tracker = AutoOpenTracker::default();
    let mut job = job();
    assert!(tracker.should_open(&job));
    assert!(!tracker.should_open(&job));
    job.received += 1;
    assert!(!tracker.should_open(&job));
    job.state = JobState::Completed;
    assert!(!tracker.should_open(&job));
    job.state = JobState::Running;
    job.last_try = Some(SystemTime::UNIX_EPOCH + Duration::from_secs(2));
    assert!(tracker.should_open(&job));
    tracker.forget(job.id);
    job.state = JobState::Completed;
    assert!(tracker.should_open(&job));
}
#[test]
fn held_jobs_do_not_open_and_failure_details_keep_copyable_recovery_paths() {
    let mut tracker = AutoOpenTracker::default();
    let mut job = job();
    job.state = JobState::Waiting;
    job.deferred = true;
    job.last_try = None;
    assert!(!tracker.should_open(&job));
    job.state = JobState::Failed;
    job.error = Some(
        JobError::new(ErrorKind::Cleanup, "Could not remove partial")
            .retained("C:/Downloads/retained.part".into()),
    );
    let view = format_transfer(&job);
    assert!(view.details.contains("C:/Downloads/retained.part"));
    assert!(view.details.contains("Could not remove partial"));
}
