use super::*;

#[test]
fn deadlines_and_budget_allow_exactly_three_additional_attempts() {
    let start = Instant::now();
    let wall = SystemTime::UNIX_EPOCH;
    let policy = RetryPolicy::default();
    let mut runtime = RetryRuntime::default();
    for attempt in 1..=3 {
        let now = start + Duration::from_secs(u64::from(attempt) * 10);
        let status = runtime.schedule(policy, now, wall).unwrap();
        assert_eq!(status.attempt, attempt);
        assert!(!runtime.due(now));
        assert!(!runtime.due(now + Duration::from_secs(4)));
        assert!(runtime.due(now + Duration::from_secs(5)));
        runtime.admitted();
        assert!(!runtime.due(now + Duration::from_secs(60)));
    }
    assert!(runtime.schedule(policy, start, wall).is_none());
    runtime.reset();
    assert_eq!(runtime.schedule(policy, start, wall).unwrap().attempt, 1);
    assert!(RetryRuntime::default()
        .schedule(
            RetryPolicy {
                max_retries: 0,
                ..policy
            },
            start,
            wall
        )
        .is_none());
}

#[test]
fn pause_freezes_the_remaining_delay_and_resume_does_not_reset_budget() {
    let start = Instant::now();
    let wall = SystemTime::UNIX_EPOCH;
    let mut runtime = RetryRuntime::default();
    let mut status = runtime
        .schedule(RetryPolicy::default(), start, wall)
        .unwrap();
    runtime.pause(start + Duration::from_secs(2), &mut status);
    assert_eq!(status.next_retry_at, None);
    assert!(!runtime.due(start + Duration::from_secs(100)));
    runtime.resume(start + Duration::from_secs(100), wall, &mut status);
    assert!(!runtime.due(start + Duration::from_secs(102)));
    assert!(runtime.due(start + Duration::from_secs(103)));
    assert_eq!(status.attempt, 1);
    runtime.admitted();
    assert_eq!(
        runtime
            .schedule(RetryPolicy::default(), start, wall)
            .unwrap()
            .attempt,
        2
    );
}

struct Harness {
    dir: tempfile::TempDir,
    queues: QueueRuntime,
    bus: EventBus,
    jobs: BTreeMap<JobId, Job>,
    waiting: VecDeque<JobId>,
    id: JobId,
}
impl Harness {
    fn new(queued: bool) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let queues = QueueRuntime::new(false);
        let bus = EventBus::new(Arc::new(|| {}));
        let mut jobs = BTreeMap::new();
        let mut waiting = VecDeque::new();
        let mut next = 1;
        let mut request = validate_request(
            "https://example.com/file",
            dir.path().join("file.bin"),
            ReplacementPolicy::Deny,
        )
        .unwrap();
        request.deferred = queued;
        let id = admit(
            request,
            &mut jobs,
            &mut waiting,
            &mut next,
            &bus,
            &queues,
            false,
        )
        .unwrap();
        waiting.clear();
        bus.drain();
        Self {
            dir,
            queues,
            bus,
            jobs,
            waiting,
            id,
        }
    }
    fn fail(&mut self, now: Instant) -> bool {
        let job = self.jobs.get_mut(&self.id).unwrap();
        job.snapshot.state = JobState::Failed;
        job.snapshot.error = Some(JobError::transient(ErrorKind::Network, "Disconnected"));
        schedule(job, now, SystemTime::UNIX_EPOCH)
    }
    fn pump(&mut self, now: Instant) {
        pump(
            &mut self.jobs,
            &mut self.waiting,
            &self.queues,
            &self.bus,
            now,
            SystemTime::UNIX_EPOCH,
        );
        self.queues
            .pump(&mut self.jobs, &mut self.waiting, &self.bus);
    }
}

#[test]
fn recovery_pump_waits_until_due_and_never_duplicates_admission() {
    let start = Instant::now();
    let mut h = Harness::new(false);
    assert!(h.fail(start));
    h.pump(start + Duration::from_secs(4));
    assert!(h.waiting.is_empty());
    assert!(!ready(&h.jobs[&h.id], &h.queues, start));
    for _ in 0..5 {
        h.pump(start + Duration::from_secs(5));
    }
    assert_eq!(h.waiting.iter().copied().collect::<Vec<_>>(), vec![h.id]);
    assert!(ready(
        &h.jobs[&h.id],
        &h.queues,
        start + Duration::from_secs(5)
    ));
    let events = h.bus.drain();
    assert!(!events
        .iter()
        .any(|event| matches!(event, Event::JobChanged(job) if job.state.is_terminal())));
    assert!(h
        .queues
        .store
        .snapshot()
        .entries
        .values()
        .all(|entry| entry.history.outcome.is_none()));
}

#[tokio::test]
async fn queue_continues_other_files_and_stop_blocks_even_an_overdue_retry() {
    let start = Instant::now();
    let mut h = Harness::new(true);
    h.queues
        .command(
            Command::StartQueue(QueueId::MAIN),
            &mut h.jobs,
            &mut h.waiting,
            &h.bus,
        )
        .await;
    assert!(h.fail(start));
    let mut request = validate_request(
        "https://example.com/other",
        h.dir.path().join("other.bin"),
        ReplacementPolicy::Deny,
    )
    .unwrap();
    request.deferred = true;
    let mut next = 2;
    let other = admit(
        request,
        &mut h.jobs,
        &mut h.waiting,
        &mut next,
        &h.bus,
        &h.queues,
        false,
    )
    .unwrap();
    h.pump(start);
    assert_eq!(h.waiting.iter().copied().collect::<Vec<_>>(), vec![other]);
    h.queues
        .command(
            Command::StopQueue(QueueId::MAIN),
            &mut h.jobs,
            &mut h.waiting,
            &h.bus,
        )
        .await;
    h.pump(start + Duration::from_secs(100));
    assert!(h.waiting.is_empty());
    assert!(!ready(
        &h.jobs[&h.id],
        &h.queues,
        start + Duration::from_secs(100)
    ));
    h.queues
        .command(
            Command::StartQueue(QueueId::MAIN),
            &mut h.jobs,
            &mut h.waiting,
            &h.bus,
        )
        .await;
    h.pump(start + Duration::from_secs(100));
    assert!(h.waiting.contains(&h.id));
}

#[test]
fn queue_parallelism_still_blocks_due_recovery() {
    let start = Instant::now();
    let mut h = Harness::new(true);
    assert!(h.fail(start));
    let mut request = validate_request(
        "https://example.com/other",
        h.dir.path().join("other.bin"),
        ReplacementPolicy::Deny,
    )
    .unwrap();
    request.deferred = true;
    let mut next = 2;
    let other = admit(
        request,
        &mut h.jobs,
        &mut h.waiting,
        &mut next,
        &h.bus,
        &h.queues,
        false,
    )
    .unwrap();
    h.jobs.get_mut(&other).unwrap().snapshot.state = JobState::Running;
    assert!(!h.queues.slot_available(&h.jobs[&h.id], &h.jobs));
}

#[test]
fn pause_resume_keeps_remaining_delay_and_shutdown_or_cancel_clear_recovery() {
    let start = Instant::now();
    for cancel in [false, true] {
        let mut h = Harness::new(false);
        assert!(h.fail(start));
        let job = h.jobs.get_mut(&h.id).unwrap();
        job.user_held = true;
        job.retry.pause(
            start + Duration::from_secs(2),
            job.snapshot.retry.as_mut().unwrap(),
        );
        h.pump(start + Duration::from_secs(100));
        assert!(h.waiting.is_empty());
        let job = h.jobs.get_mut(&h.id).unwrap();
        job.user_held = false;
        job.retry.resume(
            start + Duration::from_secs(100),
            SystemTime::UNIX_EPOCH,
            job.snapshot.retry.as_mut().unwrap(),
        );
        h.pump(start + Duration::from_secs(102));
        assert!(h.waiting.is_empty());
        let job = h.jobs.get_mut(&h.id).unwrap();
        if cancel {
            cancel_job(job, &h.bus);
        } else {
            shutdown_job(job, &h.bus);
        }
        assert!(job.snapshot.retry.is_none());
        assert!(!job.retry.due(start + Duration::from_secs(1000)));
        assert_eq!(
            job.snapshot.state,
            if cancel {
                JobState::Cancelled
            } else {
                JobState::Waiting
            }
        );
    }
}

#[test]
fn explicit_activation_replaces_timer_with_immediate_admission_and_fresh_budget() {
    let start = Instant::now();
    let mut h = Harness::new(false);
    assert!(h.fail(start));
    let job = h.jobs.get_mut(&h.id).unwrap();
    activate_external(job, &mut h.waiting, &h.bus).unwrap();
    assert!(job.snapshot.retry.is_none());
    assert!(job.snapshot.error.is_none());
    assert_eq!(h.waiting.iter().filter(|id| **id == h.id).count(), 1);
    h.waiting.clear();
    assert!(h.fail(start));
    assert_eq!(h.jobs[&h.id].snapshot.retry.as_ref().unwrap().attempt, 1);
}

#[test]
fn unsafe_failures_credentials_and_cancelled_workers_never_schedule() {
    let start = Instant::now();
    for kind in [
        ErrorKind::Io,
        ErrorKind::Storage,
        ErrorKind::Cleanup,
        ErrorKind::RestartRequired,
        ErrorKind::DestinationChanged,
    ] {
        let mut h = Harness::new(false);
        let job = h.jobs.get_mut(&h.id).unwrap();
        job.snapshot.state = JobState::Failed;
        job.snapshot.error = Some(JobError::new(kind, "permanent"));
        assert!(!schedule(job, start, SystemTime::UNIX_EPOCH));
    }
    for readiness in [
        EntryReadiness::CredentialsRequired,
        EntryReadiness::ConfirmationRequired,
        EntryReadiness::RestartRequired,
        EntryReadiness::RecoveryRequired,
    ] {
        let mut h = Harness::new(false);
        h.jobs.get_mut(&h.id).unwrap().snapshot.readiness = readiness;
        assert!(!h.fail(start));
    }
    let mut h = Harness::new(false);
    h.jobs[&h.id].cancel.token().cancel();
    assert!(!h.fail(start));
}

#[tokio::test]
async fn automatic_recovery_reopens_owned_checkpoint_without_discarding_bytes_or_identity() {
    use tokio::io::AsyncReadExt;
    let start = Instant::now();
    let mut h = Harness::new(false);
    let job = h.jobs.get_mut(&h.id).unwrap();
    let entry = job.snapshot.entry_id;
    let created = job.snapshot.created_at;
    job.request.authorization = Some(Authorization {
        login: "user".into(),
        password: "secret".into(),
    });
    let mut output = crate::output::OutputFile::durable_prepare(&job.request)
        .await
        .unwrap();
    output.write(b"saved bytes").await.unwrap();
    output.checkpoint().await.unwrap();
    let recovery = job.request.recovery.as_ref().unwrap().clone();
    let checkpoint = recovery.checkpoint().unwrap();
    drop(output);
    job.snapshot.error = Some(
        JobError::transient(ErrorKind::Network, "interrupted").retained(checkpoint.partial.clone()),
    );
    job.snapshot.state = JobState::Failed;
    assert!(schedule(job, start, SystemTime::UNIX_EPOCH));
    assert_eq!(job.snapshot.entry_id, entry);
    assert_eq!(job.snapshot.created_at, created);
    assert_eq!(
        recovery.checkpoint().unwrap().bytes_hash,
        checkpoint.bytes_hash
    );
    assert_eq!(std::fs::read(&checkpoint.partial).unwrap(), b"saved bytes");
    assert!(job.request.authorization.is_some());
    let mut output = crate::output::OutputFile::durable_prepare(&job.request)
        .await
        .unwrap();
    output.write(b" continued").await.unwrap();
    output.checkpoint().await.unwrap();
    drop(output);
    let mut file = tokio::fs::File::open(&checkpoint.partial).await.unwrap();
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).await.unwrap();
    assert_eq!(bytes, b"saved bytes continued");
}

#[tokio::test]
async fn checkpoint_storage_failure_stops_recovery_and_preserves_the_partial() {
    let mut h = Harness::new(false);
    let job = h.jobs.get_mut(&h.id).unwrap();
    let mut output = crate::output::OutputFile::durable_prepare(&job.request)
        .await
        .unwrap();
    output.write(b"keep me").await.unwrap();
    h.queues.store.set_write_failure_for_test(true);
    let error = output.checkpoint().await.unwrap_err();
    assert_eq!(error.kind, ErrorKind::Storage);
    job.snapshot.state = JobState::Failed;
    job.snapshot.error = Some(error);
    assert!(!schedule(job, Instant::now(), SystemTime::UNIX_EPOCH));
    assert!(output.partial_path().exists());
    h.queues.store.set_write_failure_for_test(false);
}

type CapturedStore = Arc<Mutex<Option<(Arc<crate::storage::Store>, EntryId)>>>;
struct FailingExecutor {
    calls: Arc<std::sync::atomic::AtomicUsize>,
    failures: usize,
    captured: CapturedStore,
}
impl TransferExecutor for FailingExecutor {
    fn execute(
        &self,
        request: DownloadRequest,
        _: TransferControl,
        _: ProgressSink,
    ) -> TransferFuture {
        let call = self.calls.fetch_add(1, Ordering::SeqCst);
        let failures = self.failures;
        let captured = self.captured.clone();
        Box::pin(async move {
            let recovery = request.recovery.as_ref().unwrap();
            *captured.lock().unwrap() = Some((recovery.store.clone(), recovery.entry));
            if call < failures {
                Err(JobError::transient(ErrorKind::Network, "Disconnected"))
            } else {
                Ok(TransferOutcome::Completed { bytes: 12 })
            }
        })
    }
}

#[test]
fn actor_publishes_only_final_history_for_success_and_exhaustion() {
    for failures in [1, 4] {
        let dir = tempfile::tempdir().unwrap();
        let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let captured: CapturedStore = Arc::new(Mutex::new(None));
        let engine = spawn_engine_with_executor(
            Arc::new(|| {}),
            Arc::new(FailingExecutor {
                calls: calls.clone(),
                failures,
                captured: captured.clone(),
            }),
        );
        let mut request = validate_request(
            "https://example.com/file",
            dir.path().join("file.bin"),
            ReplacementPolicy::Deny,
        )
        .unwrap();
        request.options.retry_policy.delay_seconds = 1;
        engine.try_send(Command::Submit(request)).unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        let mut saw_pending = false;
        let terminal = loop {
            let mut terminal = None;
            for event in engine.drain_events() {
                if let Event::JobChanged(job) = event {
                    if job.retry.is_some() {
                        saw_pending = true;
                        assert!(job.finished_at.is_none());
                    }
                    if job.state.is_terminal() {
                        terminal = Some(job);
                    }
                }
            }
            if let Some(job) = terminal {
                break job;
            }
            assert!(Instant::now() < deadline, "Missing terminal recovery event");
            std::thread::sleep(Duration::from_millis(5));
        };
        assert!(saw_pending);
        assert_eq!(
            terminal.state,
            if failures == 1 {
                JobState::Completed
            } else {
                JobState::Failed
            }
        );
        assert_eq!(
            calls.load(Ordering::SeqCst),
            if failures == 1 { 2 } else { 4 }
        );
        let (store, entry) = captured.lock().unwrap().clone().unwrap();
        assert_eq!(
            store.history(entry).unwrap().outcome,
            Some(if failures == 1 {
                crate::history::HistoryOutcome::Completed
            } else {
                crate::history::HistoryOutcome::Failed
            })
        );
        assert!(store.history(entry).unwrap().finished_at.is_some());
        engine.client().request_shutdown();
        engine.join().unwrap();
    }
}

#[test]
fn removing_a_pending_recovery_deletes_its_timer_and_history_entry() {
    let dir = tempfile::tempdir().unwrap();
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let captured: CapturedStore = Arc::new(Mutex::new(None));
    let engine = spawn_engine_with_executor(
        Arc::new(|| {}),
        Arc::new(FailingExecutor {
            calls: calls.clone(),
            failures: usize::MAX,
            captured: captured.clone(),
        }),
    );
    let mut request = validate_request(
        "https://example.com/file",
        dir.path().join("removed.bin"),
        ReplacementPolicy::Deny,
    )
    .unwrap();
    request.options.retry_policy.delay_seconds = 3600;
    engine.try_send(Command::Submit(request)).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    let id = loop {
        let pending = engine
            .drain_events()
            .into_iter()
            .find_map(|event| match event {
                Event::JobChanged(job) if job.retry.is_some() => Some(job.id),
                _ => None,
            });
        if let Some(id) = pending {
            break id;
        }
        assert!(Instant::now() < deadline, "Missing pending recovery");
        std::thread::sleep(Duration::from_millis(5));
    };
    engine.try_send(Command::Remove(id)).unwrap();
    loop {
        if engine
            .drain_events()
            .iter()
            .any(|event| matches!(event, Event::JobRemoved(removed) if *removed == id))
        {
            break;
        }
        assert!(Instant::now() < deadline, "Missing removal event");
        std::thread::sleep(Duration::from_millis(5));
    }
    let (store, entry) = captured.lock().unwrap().clone().unwrap();
    assert!(store.history(entry).is_none());
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    engine.client().request_shutdown();
    engine.join().unwrap();
}
