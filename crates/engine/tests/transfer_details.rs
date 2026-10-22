use rusty_engine::{
    service::{spawn_engine_with_executor, TransferExecutor, TransferFuture},
    transfer::{ProgressSink, TransferControl, TransferOutcome, TransferProgress},
    *,
};
use std::{
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

struct Retried(std::sync::atomic::AtomicUsize);
impl TransferExecutor for Retried {
    fn execute(
        &self,
        _request: DownloadRequest,
        _control: TransferControl,
        sink: ProgressSink,
    ) -> TransferFuture {
        use std::sync::atomic::Ordering;
        let attempt = self.0.fetch_add(1, Ordering::SeqCst);
        Box::pin(async move {
            let received = if attempt == 0 { 10 } else { 100 };
            sink(TransferProgress {
                received,
                total: Some(100),
                resume_capable: true,
                paused: false,
                details: Some(ConnectionProgress {
                    connection_limit: 2,
                    segments: vec![SegmentProgress {
                        id: if attempt == 0 { 9 } else { 0 },
                        start: 0,
                        end: Some(99),
                        received,
                        transferred: received,
                        generation: 1,
                        state: SegmentState::Receiving,
                    }],
                }),
            });
            if attempt == 0 {
                Err(JobError::transient(
                    ErrorKind::Network,
                    "Test connection interrupted",
                ))
            } else {
                // Confirmation/finalization can report aggregate progress only,
                // after the last worker update but before the next actor tick.
                sink(TransferProgress {
                    received: 100,
                    total: Some(100),
                    resume_capable: true,
                    paused: true,
                    details: None,
                });
                Ok(TransferOutcome::Completed { bytes: 100 })
            }
        })
    }
}

#[test]
fn automatic_retry_replaces_worker_rows_and_discards_old_speeds() {
    let directory = tempfile::tempdir().unwrap();
    let engine = spawn_engine_with_executor(Arc::new(|| {}), Arc::new(Retried(Default::default())));
    let mut request = validate_request(
        "https://example.test/retry.bin",
        directory.path().join("retry.bin"),
        ReplacementPolicy::Deny,
    )
    .unwrap();
    request.options.retry_policy.delay_seconds = 1;
    engine.try_send(Command::Submit(request)).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut retry_seen = false;
    let done = loop {
        let mut result = None;
        for event in engine.drain_events() {
            if let Event::JobChanged(job) = event {
                if job.retry.is_some() {
                    retry_seen = true;
                    assert_eq!(job.bytes_per_second, 0);
                    assert_eq!(job.transfer_details.as_ref().unwrap().active_connections, 0);
                    assert_eq!(
                        job.transfer_details.as_ref().unwrap().segments[0].state,
                        SegmentState::Failed
                    );
                }
                if job.state.is_terminal() {
                    result = Some(job);
                }
            }
        }
        if let Some(job) = result {
            break job;
        }
        assert!(
            Instant::now() < deadline,
            "retry telemetry job did not finish"
        );
        thread::sleep(Duration::from_millis(2));
    };
    assert!(retry_seen);
    assert_eq!(done.state, JobState::Completed);
    let details = done.transfer_details.unwrap();
    assert_eq!(details.segments.len(), 1);
    assert_eq!(details.segments[0].id, 0);
    assert_eq!(details.segments[0].bytes_per_second, 0);
    engine.try_send(Command::Shutdown).unwrap();
    engine.join().unwrap();
}

struct Immediate {
    telemetry: bool,
    cancel: bool,
}
impl TransferExecutor for Immediate {
    fn execute(
        &self,
        _request: DownloadRequest,
        control: TransferControl,
        sink: ProgressSink,
    ) -> TransferFuture {
        let telemetry = self.telemetry;
        let cancel = self.cancel;
        Box::pin(async move {
            sink(TransferProgress {
                received: 100,
                total: Some(100),
                resume_capable: true,
                paused: false,
                details: telemetry.then(|| ConnectionProgress {
                    connection_limit: 4,
                    segments: vec![SegmentProgress {
                        id: 9,
                        start: 0,
                        end: Some(99),
                        received: 100,
                        transferred: 100,
                        generation: 1,
                        state: SegmentState::Receiving,
                    }],
                }),
            });
            if cancel {
                control.token().cancelled().await;
                Ok(TransferOutcome::Cancelled)
            } else {
                sink(TransferProgress {
                    received: 100,
                    total: Some(100),
                    resume_capable: true,
                    paused: true,
                    details: None,
                });
                Ok(TransferOutcome::Completed { bytes: 100 })
            }
        })
    }
}

fn run(telemetry: bool, cancel: bool) -> JobSnapshot {
    let directory = tempfile::tempdir().unwrap();
    let engine =
        spawn_engine_with_executor(Arc::new(|| {}), Arc::new(Immediate { telemetry, cancel }));
    let request = validate_request(
        "https://example.test/file.bin",
        directory.path().join("file.bin"),
        ReplacementPolicy::Deny,
    )
    .unwrap();
    engine.try_send(Command::Submit(request)).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut cancelled = false;
    let final_job = loop {
        let mut result = None;
        for event in engine.drain_events() {
            if let Event::JobChanged(job) = event {
                if cancel && !cancelled && job.transfer_details.is_some() {
                    engine.try_send(Command::Cancel(job.id)).unwrap();
                    cancelled = true;
                }
                if job.state.is_terminal() {
                    result = Some(job);
                }
            }
        }
        if let Some(job) = result {
            break job;
        }
        assert!(Instant::now() < deadline, "telemetry job did not finish");
        thread::sleep(Duration::from_millis(2));
    };
    engine.try_send(Command::Shutdown).unwrap();
    engine.join().unwrap();
    final_job
}

#[test]
fn final_progress_survives_completion_before_the_next_actor_tick() {
    let done = run(true, false);
    assert_eq!(done.state, JobState::Completed);
    let details = done.transfer_details.unwrap();
    assert_eq!(details.connection_limit, 4);
    assert_eq!(details.active_connections, 0);
    assert_eq!(details.segments[0].id, 9);
    assert_eq!(details.segments[0].received, 100);
    assert_eq!(details.segments[0].state, SegmentState::Completed);
    assert_eq!(details.segments[0].bytes_per_second, 0);
}

#[test]
fn cancellation_retains_segment_bytes_without_stale_connections() {
    let done = run(true, true);
    assert_eq!(done.state, JobState::Cancelled);
    let details = done.transfer_details.unwrap();
    assert_eq!(details.active_connections, 0);
    assert_eq!(details.segments[0].received, 100);
    assert_eq!(details.segments[0].state, SegmentState::Cancelled);
    assert_eq!(details.segments[0].bytes_per_second, 0);
}

#[test]
fn unsupported_backends_do_not_invent_a_single_connection() {
    assert!(run(false, false).transfer_details.is_none());
}
