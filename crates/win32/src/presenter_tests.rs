use super::*;
use std::path::PathBuf;

fn snapshot(id: u64, filename: &str, state: JobState) -> JobSnapshot {
    JobSnapshot {
        entry_id: None,
        restored: false,
        recovery_paths: vec![],
        queue_id: None,
        readiness: rusty_engine::EntryReadiness::Ready,
        deferred: false,
        category: None,
        description: String::new(),
        id: JobId(id),
        filename: filename.into(),
        source_host: "example.com".into(),
        display_url: "https://example.com/file".into(),
        destination: PathBuf::from(filename),
        state,
        received: 0,
        total: None,
        resume_capable: false,
        speed_limit: None,
        bytes_per_second: 0,
        eta_seconds: None,
        last_try: None,
        error: None,
    }
}
#[test]
fn paused_download_can_resume_stop_and_stop_all() {
    let mut view = ViewModel::default();
    view.apply(Event::JobChanged(snapshot(1, "file", JobState::Paused)));
    let actions = view.actions(Some(JobId(1)));
    assert!(actions.start && actions.stop && actions.stop_all);
    assert!(!actions.retry);
    assert!(view.active());
}
#[test]
fn filters_preserve_identity_and_classify_case_insensitively() {
    let mut view = ViewModel::default();
    view.apply(Event::JobChanged(snapshot(5, "z.ZIP", JobState::Completed)));
    view.apply(Event::JobChanged(snapshot(2, "a.pdf", JobState::Failed)));
    view.apply(Event::JobChanged(snapshot(
        8,
        "other.xyz",
        JobState::Cancelled,
    )));
    assert_eq!(view.visible_jobs(Category::Compressed), vec![JobId(5)]);
    assert_eq!(view.visible_jobs(Category::Documents), vec![JobId(2)]);
    assert_eq!(view.visible_jobs(Category::Finished), vec![JobId(5)]);
    assert_eq!(
        view.visible_jobs(Category::Unfinished),
        vec![JobId(2), JobId(8)]
    );
    view.apply(Event::JobRemoved(JobId(2)));
    assert_eq!(view.visible_jobs(Category::All), vec![JobId(5), JobId(8)]);
}
#[test]
fn unknown_totals_have_no_percentage_or_eta() {
    let mut s = snapshot(1, "file", JobState::Running);
    s.received = 100;
    s.bytes_per_second = 10;
    s.eta_seconds = Some(100);
    let row = format_snapshot(&s);
    assert!(!row.columns[2].contains('%'));
    assert!(row.columns[3].is_empty());
    assert!(row.columns[5].is_empty());
    s.state = JobState::Completed;
    s.received = 0;
    s.total = Some(0);
    assert_eq!(format_snapshot(&s).columns[1], "0 B");
    assert_eq!(format_snapshot(&s).columns[2], "Completed");
}
#[test]
fn stale_selection_cannot_retry_remove_or_open() {
    let mut view = ViewModel::default();
    assert!(!view.actions(Some(JobId(99))).remove);
    view.apply(Event::JobChanged(snapshot(1, "a", JobState::Running)));
    let actions = view.actions(Some(JobId(1)));
    assert!(actions.stop);
    assert!(actions.remove);
    assert!(!actions.retry);
    assert!(!actions.open_file);
    view.apply(Event::JobChanged(snapshot(1, "a", JobState::Failed)));
    assert!(view.actions(Some(JobId(1))).retry);
    view.apply(Event::JobRemoved(JobId(1)));
    assert!(!view.actions(Some(JobId(1))).retry);
}

#[test]
fn shutdown_report_preserves_failed_cleanup_paths_until_user_acknowledgment() {
    let mut view = ViewModel::default();
    let mut job = snapshot(1, "file.bin", JobState::Failed);
    job.error = Some(
        rusty_engine::JobError::new(rusty_engine::ErrorKind::Cleanup, "Cleanup failed")
            .retained(PathBuf::from("C:/Downloads/தமிழ்.part")),
    );
    view.apply(Event::JobChanged(job));
    view.apply(Event::ShutdownComplete);
    let report = view.recovery_notice().unwrap();
    assert!(report.contains("C:/Downloads/தமிழ்.part"));
    assert!(report.contains("Cleanup failed"));
    assert!(!view.actions(Some(JobId(1))).retry);
    assert_eq!(view.jobs.len(), 1);
}
