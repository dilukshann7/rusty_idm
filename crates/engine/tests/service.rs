use rusty_engine::{
    service::{spawn_engine_with_executor, TransferExecutor, TransferFuture},
    transfer::{ProgressSink, TransferControl, TransferOutcome},
    *,
};
use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

struct Held {
    started: Arc<AtomicUsize>,
    cleanup_gate: Option<Arc<tokio::sync::Semaphore>>,
}
impl TransferExecutor for Held {
    fn execute(
        &self,
        request: DownloadRequest,
        cancel: TransferControl,
        progress: ProgressSink,
    ) -> TransferFuture {
        let started = self.started.clone();
        let cleanup_gate = self.cleanup_gate.clone();
        Box::pin(async move {
            let output = rusty_engine::output::OutputFile::prepare(
                &request.destination,
                &request.replacement,
            )
            .await?;
            started.fetch_add(1, Ordering::SeqCst);
            progress(rusty_engine::transfer::TransferProgress {
                received: 5,
                total: None,
                resume_capable: false,
                paused: false,
            });
            cancel.token().cancelled().await;
            // Model cleanup that must finish before admission/removal/shutdown.
            if let Some(gate) = cleanup_gate {
                gate.acquire().await.unwrap().forget();
            } else {
                tokio::time::sleep(Duration::from_millis(40)).await;
            }
            output.abort().await?;
            Ok(TransferOutcome::Cancelled)
        })
    }
}
fn request(name: impl Into<std::path::PathBuf>) -> DownloadRequest {
    let mut request = validate_request(
        "https://example.com/file",
        name.into(),
        ReplacementPolicy::Deny,
    )
    .unwrap();
    // These admission tests deliberately submit independent copies of one URL.
    request.allow_duplicate = true;
    request
}
fn collect_until(engine: &EngineHandle, check: impl Fn(&[Event]) -> bool) -> Vec<Event> {
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut events = vec![];
    loop {
        events.extend(engine.drain_events());
        if check(&events) {
            return events;
        }
        assert!(Instant::now() < deadline, "event deadline: {events:?}");
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn duplicate_lookup_stays_current_after_drain_and_default_submission_is_not_duplicated() {
    let dir = tempfile::tempdir().unwrap();
    let started = Arc::new(AtomicUsize::new(0));
    let engine = spawn_engine_with_executor(
        Arc::new(|| {}),
        Arc::new(Held {
            started: started.clone(),
            cleanup_gate: None,
        }),
    );
    let mut first = request(dir.path().join("first.bin"));
    first.allow_duplicate = false;
    engine.try_send(Command::Submit(first.clone())).unwrap();
    collect_until(&engine, |events| {
        events
            .iter()
            .any(|e| matches!(e, Event::JobChanged(s) if s.state == JobState::Running))
    });
    assert_eq!(engine.find_duplicate(&first).unwrap().id, JobId(1));
    let mut duplicate = first.clone();
    duplicate.destination = dir.path().join("other.bin");
    engine.try_send(Command::Submit(duplicate.clone())).unwrap();
    collect_until(&engine, |events| {
        events
            .iter()
            .any(|e| matches!(e, Event::CommandRejected { .. }))
    });
    assert_eq!(started.load(Ordering::SeqCst), 1);
    assert_eq!(engine.destinations(), vec![first.destination.clone()]);
    engine.try_send(Command::Activate(JobId(1))).unwrap();
    engine.try_send(Command::Remove(JobId(1))).unwrap();
    collect_until(&engine, |events| {
        events
            .iter()
            .any(|e| matches!(e, Event::JobRemoved(JobId(1))))
    });
    assert!(engine.find_duplicate(&duplicate).is_none());
    engine.try_send(Command::Shutdown).unwrap();
    collect_until(&engine, |events| {
        events.iter().any(|e| matches!(e, Event::ShutdownComplete))
    });
    engine.join().unwrap();
}

#[test]
fn overwrite_running_entry_restarts_same_id_only_after_cleanup() {
    let dir = tempfile::tempdir().unwrap();
    let started = Arc::new(AtomicUsize::new(0));
    let gate = Arc::new(tokio::sync::Semaphore::new(0));
    let engine = spawn_engine_with_executor(
        Arc::new(|| {}),
        Arc::new(Held {
            started: started.clone(),
            cleanup_gate: Some(gate.clone()),
        }),
    );
    let original = request(dir.path().join("file.bin"));
    engine.try_send(Command::Submit(original.clone())).unwrap();
    collect_until(&engine, |_| started.load(Ordering::SeqCst) == 1);
    engine.try_send(Command::Restart(JobId(1))).unwrap();
    collect_until(&engine, |events| {
        events
            .iter()
            .any(|e| matches!(e, Event::JobChanged(s) if s.state == JobState::Cancelling))
    });
    assert_eq!(started.load(Ordering::SeqCst), 1);
    gate.add_permits(1);
    collect_until(&engine, |_| started.load(Ordering::SeqCst) == 2);
    assert_eq!(engine.find_duplicate(&original).unwrap().id, JobId(1));
    assert_eq!(engine.destinations().len(), 1);
    engine.try_send(Command::Shutdown).unwrap();
    gate.add_permits(1);
    collect_until(&engine, |events| {
        events.iter().any(|e| matches!(e, Event::ShutdownComplete))
    });
    engine.join().unwrap();
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
}

#[test]
fn cancel_all_cancels_a_pending_overwrite_restart() {
    let dir = tempfile::tempdir().unwrap();
    let started = Arc::new(AtomicUsize::new(0));
    let gate = Arc::new(tokio::sync::Semaphore::new(0));
    let engine = spawn_engine_with_executor(
        Arc::new(|| {}),
        Arc::new(Held {
            started: started.clone(),
            cleanup_gate: Some(gate.clone()),
        }),
    );
    engine
        .try_send(Command::Submit(request(dir.path().join("file.bin"))))
        .unwrap();
    collect_until(&engine, |_| started.load(Ordering::SeqCst) == 1);
    engine.try_send(Command::Restart(JobId(1))).unwrap();
    collect_until(&engine, |events| {
        events
            .iter()
            .any(|e| matches!(e, Event::JobChanged(s) if s.state == JobState::Cancelling))
    });
    engine.try_send(Command::CancelAll).unwrap();
    gate.add_permits(1);
    collect_until(&engine, |events| {
        events
            .iter()
            .any(|e| matches!(e, Event::JobChanged(s) if s.state == JobState::Cancelled))
    });
    assert_eq!(started.load(Ordering::SeqCst), 1);
    engine.try_send(Command::Shutdown).unwrap();
    collect_until(&engine, |events| {
        events.iter().any(|e| matches!(e, Event::ShutdownComplete))
    });
    engine.join().unwrap();
}
#[test]
fn two_running_one_waiting_and_waiting_cancel_never_executes() {
    let dir = tempfile::tempdir().unwrap();
    let started = Arc::new(AtomicUsize::new(0));
    let engine = spawn_engine_with_executor(
        Arc::new(|| {}),
        Arc::new(Held {
            started: started.clone(),
            cleanup_gate: None,
        }),
    );
    for name in ["a.bin", "b.bin", "c.bin"] {
        engine
            .try_send(Command::Submit(request(dir.path().join(name))))
            .unwrap();
    }
    collect_until(&engine, |events| {
        events
            .iter()
            .filter(|e| matches!(e, Event::JobChanged(s) if s.state == JobState::Running))
            .count()
            == 2 && started.load(Ordering::SeqCst) == 2
            && events.iter().any(|e| matches!(e, Event::JobChanged(s) if s.id == JobId(3) && s.state == JobState::Waiting))
    });
    engine.try_send(Command::Cancel(JobId(3))).unwrap();
    collect_until(&engine, |events| {
        events.iter().any(|e| matches!(e, Event::JobChanged(s) if s.id == JobId(3) && s.state == JobState::Cancelled))
    });
    assert_eq!(started.load(Ordering::SeqCst), 2);
    engine.try_send(Command::Shutdown).unwrap();
    collect_until(&engine, |events| {
        events.iter().any(|e| matches!(e, Event::ShutdownComplete))
    });
    engine.join().unwrap();
}
#[test]
fn removal_waits_for_cleanup_and_shutdown_delivers_terminals_first() {
    let dir = tempfile::tempdir().unwrap();
    let engine = spawn_engine_with_executor(
        Arc::new(|| {}),
        Arc::new(Held {
            started: Arc::new(AtomicUsize::new(0)),
            cleanup_gate: None,
        }),
    );
    engine
        .try_send(Command::Submit(request(dir.path().join("a.bin"))))
        .unwrap();
    collect_until(&engine, |events| {
        events
            .iter()
            .any(|e| matches!(e, Event::JobChanged(s) if s.state == JobState::Running))
    });
    engine.try_send(Command::Remove(JobId(1))).unwrap();
    let events = collect_until(&engine, |events| {
        events
            .iter()
            .any(|e| matches!(e, Event::JobRemoved(JobId(1))))
    });
    let terminal = events
        .iter()
        .position(|e| matches!(e, Event::JobChanged(s) if s.state == JobState::Cancelled))
        .unwrap();
    let removed = events
        .iter()
        .position(|e| matches!(e, Event::JobRemoved(_)))
        .unwrap();
    assert!(terminal < removed);
    engine.try_send(Command::Shutdown).unwrap();
    assert!(engine
        .try_send(Command::Submit(request(dir.path().join("late.bin"))))
        .is_err());
    collect_until(&engine, |events| {
        events.iter().any(|e| matches!(e, Event::ShutdownComplete))
    });
    engine.join().unwrap();
}
#[test]
fn retry_revalidates_destination_and_does_not_reuse_received_count() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("retry.bin");
    let engine = spawn_engine_with_executor(
        Arc::new(|| {}),
        Arc::new(Held {
            started: Arc::new(AtomicUsize::new(0)),
            cleanup_gate: None,
        }),
    );
    let req = validate_request(
        "https://example.com/file",
        target.clone(),
        ReplacementPolicy::Deny,
    )
    .unwrap();
    engine.try_send(Command::Submit(req)).unwrap();
    collect_until(&engine, |e| {
        e.iter()
            .any(|e| matches!(e, Event::JobChanged(s) if s.state == JobState::Running))
    });
    engine.try_send(Command::Cancel(JobId(1))).unwrap();
    collect_until(&engine, |e| {
        e.iter()
            .any(|e| matches!(e, Event::JobChanged(s) if s.state == JobState::Cancelled))
    });
    std::fs::write(&target, b"external").unwrap();
    engine.try_send(Command::Retry(JobId(1))).unwrap();
    collect_until(&engine, |e| {
        e.iter().any(
            |e| matches!(e, Event::JobChanged(s) if s.state == JobState::Failed && s.received == 0),
        )
    });
    assert_eq!(std::fs::read(&target).unwrap(), b"external");
    engine.try_send(Command::Shutdown).unwrap();
    collect_until(&engine, |e| {
        e.iter().any(|e| matches!(e, Event::ShutdownComplete))
    });
    engine.join().unwrap();
}

#[test]
fn admission_slot_stays_occupied_until_cleanup_releases_it() {
    let dir = tempfile::tempdir().unwrap();
    let started = Arc::new(AtomicUsize::new(0));
    let gate = Arc::new(tokio::sync::Semaphore::new(0));
    let engine = spawn_engine_with_executor(
        Arc::new(|| {}),
        Arc::new(Held {
            started: started.clone(),
            cleanup_gate: Some(gate.clone()),
        }),
    );
    for name in ["a", "b", "c"] {
        engine
            .try_send(Command::Submit(request(dir.path().join(name))))
            .unwrap();
    }
    collect_until(&engine, |_| started.load(Ordering::SeqCst) == 2);
    engine.try_send(Command::Cancel(JobId(1))).unwrap();
    collect_until(&engine, |events| {
        events.iter().any(|e| matches!(e, Event::JobChanged(s) if s.id == JobId(1) && s.state == JobState::Cancelling))
    });
    assert_eq!(started.load(Ordering::SeqCst), 2);
    gate.add_permits(1);
    collect_until(&engine, |_| started.load(Ordering::SeqCst) == 3);
    engine.try_send(Command::Shutdown).unwrap();
    gate.add_permits(2);
    let events = collect_until(&engine, |events| {
        events.iter().any(|e| matches!(e, Event::ShutdownComplete))
    });
    let done = events
        .iter()
        .position(|e| matches!(e, Event::ShutdownComplete))
        .unwrap();
    assert!(events
        .iter()
        .enumerate()
        .filter(|(_, e)| matches!(e, Event::JobChanged(s) if s.state.is_terminal()))
        .all(|(index, _)| index < done));
    engine.join().unwrap();
}

struct FailImmediately;
impl TransferExecutor for FailImmediately {
    fn execute(&self, _: DownloadRequest, _: TransferControl, _: ProgressSink) -> TransferFuture {
        Box::pin(async { Err(JobError::new(ErrorKind::Http, "Fixture failure")) })
    }
}
fn send_eventually(engine: &EngineHandle, mut command: Command) {
    // Recreate only a nonaccepted command on QueueFull; never duplicate accepted submissions.
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        let retry = match &command {
            Command::Submit(request) => Command::Submit(request.clone()),
            _ => panic!("test helper only submits"),
        };
        match engine.try_send(command) {
            Ok(()) => return,
            Err(CommandError::QueueFull) => {
                command = retry;
                assert!(Instant::now() < deadline);
                std::thread::sleep(Duration::from_millis(1));
            }
            Err(error) => panic!("unexpected command failure: {error}"),
        }
    }
}
#[test]
fn retained_entry_limit_rejects_visibly_without_losing_shutdown() {
    let dir = tempfile::tempdir().unwrap();
    let engine = spawn_engine_with_executor(Arc::new(|| {}), Arc::new(FailImmediately));
    for index in 0..1001 {
        send_eventually(
            &engine,
            Command::Submit(request(dir.path().join(format!("file-{index}.bin")))),
        );
    }
    let events = collect_until(&engine, |events| {
        events
            .iter()
            .any(|e| matches!(e, Event::CommandRejected { .. }))
    });
    let ids: std::collections::BTreeSet<_> = events
        .iter()
        .filter_map(|e| {
            if let Event::JobChanged(s) = e {
                Some(s.id)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(ids.len(), 1000);
    engine.try_send(Command::Shutdown).unwrap();
    collect_until(&engine, |events| {
        events.iter().any(|e| matches!(e, Event::ShutdownComplete))
    });
    engine.join().unwrap();
}

struct CleanupFailure {
    partial: std::path::PathBuf,
}
impl TransferExecutor for CleanupFailure {
    fn execute(
        &self,
        _: DownloadRequest,
        control: TransferControl,
        _: ProgressSink,
    ) -> TransferFuture {
        let partial = self.partial.clone();
        Box::pin(async move {
            control.token().cancelled().await;
            Err(JobError::new(ErrorKind::Cleanup, "Partial cleanup failed").retained(partial))
        })
    }
}
#[test]
fn active_removal_preserves_cleanup_failure_and_shutdown_delivers_its_path() {
    let dir = tempfile::tempdir().unwrap();
    let partial = dir.path().join("retained.part");
    std::fs::write(&partial, b"recovery").unwrap();
    let engine = spawn_engine_with_executor(
        Arc::new(|| {}),
        Arc::new(CleanupFailure {
            partial: partial.clone(),
        }),
    );
    engine
        .try_send(Command::Submit(request(dir.path().join("file.bin"))))
        .unwrap();
    collect_until(&engine, |e| {
        e.iter()
            .any(|e| matches!(e, Event::JobChanged(s) if s.state == JobState::Running))
    });
    engine.try_send(Command::Remove(JobId(1))).unwrap();
    let events = collect_until(&engine, |e| {
        e.iter()
            .any(|e| matches!(e, Event::JobChanged(s) if s.state == JobState::Failed))
    });
    assert!(!events.iter().any(|e| matches!(e, Event::JobRemoved(_))));
    assert!(events.iter().any(|e| matches!(e, Event::JobChanged(s) if s.error.as_ref().and_then(|e| e.retained_partial.as_ref()) == Some(&partial))));
    engine.try_send(Command::Retry(JobId(1))).unwrap();
    collect_until(&engine, |e| {
        e.iter().any(|e| matches!(e, Event::CommandRejected { .. }))
    });
    engine.try_send(Command::Shutdown).unwrap();
    collect_until(&engine, |e| {
        e.iter().any(|e| matches!(e, Event::ShutdownComplete))
    });
    engine.join().unwrap();
    assert_eq!(std::fs::read(partial).unwrap(), b"recovery");
}
#[test]
fn shutdown_orders_a_new_cleanup_failure_before_shutdown_complete() {
    let dir = tempfile::tempdir().unwrap();
    let partial = dir.path().join("retained.part");
    let engine = spawn_engine_with_executor(
        Arc::new(|| {}),
        Arc::new(CleanupFailure {
            partial: partial.clone(),
        }),
    );
    engine
        .try_send(Command::Submit(request(dir.path().join("file.bin"))))
        .unwrap();
    collect_until(&engine, |e| {
        e.iter()
            .any(|e| matches!(e, Event::JobChanged(s) if s.state == JobState::Running))
    });
    engine.try_send(Command::Shutdown).unwrap();
    let events = collect_until(&engine, |e| {
        e.iter().any(|e| matches!(e, Event::ShutdownComplete))
    });
    let failure = events.iter().position(|e| matches!(e, Event::JobChanged(s) if s.error.as_ref().and_then(|e| e.retained_partial.as_ref()) == Some(&partial))).unwrap();
    let complete = events
        .iter()
        .position(|e| matches!(e, Event::ShutdownComplete))
        .unwrap();
    assert!(failure < complete);
    engine.join().unwrap();
}

#[test]
fn download_later_never_executes_until_explicitly_started() {
    let dir = tempfile::tempdir().unwrap();
    let started = Arc::new(AtomicUsize::new(0));
    let engine = spawn_engine_with_executor(
        Arc::new(|| {}),
        Arc::new(Held {
            started: started.clone(),
            cleanup_gate: None,
        }),
    );
    let mut later = request(dir.path().join("later.bin"));
    later.deferred = true;
    engine.try_send(Command::Submit(later)).unwrap();
    let events = collect_until(&engine, |events| {
        events
            .iter()
            .any(|e| matches!(e, Event::JobChanged(s) if s.deferred))
    });
    assert!(events
        .iter()
        .all(|e| !matches!(e, Event::JobChanged(s) if s.state == JobState::Running)));
    assert_eq!(started.load(Ordering::SeqCst), 0);
    engine.try_send(Command::Start(JobId(1))).unwrap();
    collect_until(&engine, |_| started.load(Ordering::SeqCst) == 1);
    engine.try_send(Command::Shutdown).unwrap();
    collect_until(&engine, |e| {
        e.iter().any(|e| matches!(e, Event::ShutdownComplete))
    });
    engine.join().unwrap();
}
