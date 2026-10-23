use super::*;
use crate::{history::HistoryOutcome, storage::Store};

fn request(dir: &std::path::Path, name: &str) -> DownloadRequest {
    let mut request = validate_request(
        "https://example.com/download?token=private",
        dir.join(name),
        ReplacementPolicy::Deny,
    )
    .unwrap();
    request.category = Some("Documents".into());
    request.description = "My saved download".into();
    request.source_page = Some("https://example.com/source".into());
    request
}

type CapturedStore = Arc<Mutex<Option<(Arc<Store>, EntryId)>>>;

#[test]
fn full_history_rejects_admission_without_pruning_or_starting_jobs() {
    let dir = tempfile::tempdir().unwrap();
    let queues = QueueRuntime::new(false);
    queues
        .store
        .update(|state| {
            for index in 1..=10_000 {
                let id = EntryId(index);
                let request = request(dir.path(), &format!("{index}.bin"));
                state
                    .entries
                    .insert(id, queue_support::stored(id, &request));
            }
            state.next_entry = 10_001;
            Ok(())
        })
        .unwrap();
    let bus = EventBus::new(Arc::new(|| {}));
    let mut jobs = BTreeMap::new();
    let mut waiting = VecDeque::new();
    let mut next = 1;
    // Populate actor records directly: restoring 10,000 entries would exercise
    // unrelated startup work instead of this admission boundary.
    let template_request = request(dir.path(), "held.bin");
    let template_bus = EventBus::new(Arc::new(|| {}));
    let mut template_jobs = BTreeMap::new();
    let mut template_waiting = VecDeque::new();
    let template_queues = QueueRuntime::new(false);
    admit(
        template_request,
        &mut template_jobs,
        &mut template_waiting,
        &mut next,
        &template_bus,
        &template_queues,
        false,
    )
    .unwrap();
    let template = template_jobs.remove(&JobId(1)).unwrap();
    for index in 1..=10_000 {
        let mut snapshot = template.snapshot.clone();
        snapshot.id = JobId(index);
        jobs.insert(
            snapshot.id,
            Job {
                transfer_meter: Default::default(),
                retry: Default::default(),
                request: template.request.clone(),
                retired_history: None,
                snapshot,
                cancel: TransferControl::new(CancellationToken::new()),
                remove: false,
                speed_meter: SpeedMeter::new(Instant::now(), 0),
                started: false,
                restart: false,
                resume_after_pause: false,
                user_held: true,
            },
        );
    }
    bus.drain();
    assert_eq!(jobs.len(), 10_000);
    assert!(admit(
        request(dir.path(), "extra.bin"),
        &mut jobs,
        &mut waiting,
        &mut next,
        &bus,
        &queues,
        false,
    )
    .is_err());
    assert_eq!(jobs.len(), 10_000);
    assert_eq!(queues.store.snapshot().entries.len(), 10_000);
    assert!(waiting.is_empty());
    let events = bus.drain();
    assert!(events.iter().any(|event| matches!(event,
        Event::CommandRejected { message } if message.contains("history is full"))));
    assert!(!events
        .iter()
        .any(|event| matches!(event, Event::JobRemoved(_))));
}

struct HistoryExecutor {
    captured: CapturedStore,
    fail: bool,
    block_final_save: bool,
}
impl TransferExecutor for HistoryExecutor {
    fn execute(
        &self,
        request: DownloadRequest,
        _: TransferControl,
        _: ProgressSink,
    ) -> TransferFuture {
        let captured = self.captured.clone();
        let fail = self.fail;
        let block_final_save = self.block_final_save;
        Box::pin(async move {
            let recovery = request.recovery.as_ref().unwrap();
            *captured.lock().unwrap() = Some((recovery.store.clone(), recovery.entry));
            if block_final_save {
                recovery.store.set_write_failure_for_test(true);
            }
            if fail {
                Err(JobError::new(ErrorKind::Network, "secret server detail"))
            } else {
                Ok(TransferOutcome::Completed { bytes: 73 })
            }
        })
    }
}

#[test]
fn actor_persists_fake_executor_results_before_publishing_terminal_events() {
    let dir = tempfile::tempdir().unwrap();
    for fail in [false, true] {
        let captured: CapturedStore = Arc::new(Mutex::new(None));
        let engine = spawn_engine_with_executor(
            Arc::new(|| {}),
            Arc::new(HistoryExecutor {
                captured: captured.clone(),
                fail,
                block_final_save: false,
            }),
        );
        engine
            .try_send(Command::Submit(request(
                dir.path(),
                if fail { "fail.bin" } else { "success.bin" },
            )))
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        let terminal = loop {
            if let Some(snapshot) =
                engine
                    .drain_events()
                    .into_iter()
                    .find_map(|event| match event {
                        Event::JobChanged(snapshot) if snapshot.state.is_terminal() => {
                            Some(snapshot)
                        }
                        _ => None,
                    })
            {
                break snapshot;
            }
            assert!(Instant::now() < deadline, "No terminal history event");
            std::thread::sleep(Duration::from_millis(5));
        };
        assert!(
            terminal.created_at.is_some()
                && terminal.last_try.is_some()
                && terminal.finished_at.is_some()
        );
        let (store, entry) = captured.lock().unwrap().clone().unwrap();
        assert_eq!(
            store.history(entry).unwrap().outcome,
            Some(if fail {
                HistoryOutcome::Failed
            } else {
                HistoryOutcome::Completed
            })
        );
        engine.client().request_shutdown();
        engine.join().unwrap();
        let (_queues, restored, _) = restore(store);
        let snapshot = &restored.values().next().unwrap().snapshot;
        assert_eq!(snapshot.state, terminal.state);
        assert_eq!(snapshot.received, if fail { 0 } else { 73 });
    }
}

fn restore(store: Arc<Store>) -> (QueueRuntime, BTreeMap<JobId, Job>, EventBus) {
    let mut queues = QueueRuntime::new(false);
    queues.store = store.reloaded_for_test();
    let bus = EventBus::new(Arc::new(|| {}));
    let mut jobs = BTreeMap::new();
    queues.restore(&mut jobs, &mut 100, &bus);
    (queues, jobs, bus)
}

#[test]
fn ordinary_failure_survives_restart_with_dates_category_error_and_duplicate_identity() {
    let dir = tempfile::tempdir().unwrap();
    let queues = QueueRuntime::new(false);
    let bus = EventBus::new(Arc::new(|| {}));
    let mut jobs = BTreeMap::new();
    let request = request(dir.path(), "failed.bin");
    let id = admit(
        request.clone(),
        &mut jobs,
        &mut VecDeque::new(),
        &mut 1,
        &bus,
        &queues,
        false,
    )
    .unwrap();
    let job = jobs.get_mut(&id).unwrap();
    let entry = job.snapshot.entry_id;
    let added = job.snapshot.created_at;
    job.snapshot.state = JobState::Running;
    job.snapshot.last_try = Some(SystemTime::UNIX_EPOCH + Duration::from_secs(10));
    publish_job(job, &bus);
    job.snapshot.state = JobState::Failed;
    job.snapshot.error = Some(JobError::new(
        ErrorKind::Network,
        "secret Authorization: private",
    ));
    publish_job(job, &bus);
    let finished = job.snapshot.finished_at;
    let (_queues, restored, restored_bus) = restore(queues.store.clone());
    let job = restored.values().next().unwrap();
    assert_eq!(job.snapshot.state, JobState::Failed);
    assert_eq!(job.snapshot.entry_id, entry);
    assert_eq!(job.snapshot.created_at, added);
    assert_eq!(job.snapshot.finished_at, finished);
    assert_eq!(
        job.snapshot.last_try,
        Some(SystemTime::UNIX_EPOCH + Duration::from_secs(10))
    );
    assert_eq!(job.snapshot.received, 0);
    assert_eq!(job.snapshot.total, None);
    assert_eq!(job.snapshot.category.as_deref(), Some("Documents"));
    assert_eq!(job.snapshot.description, "My saved download");
    assert_eq!(job.request.source_page, request.source_page);
    assert!(!job
        .snapshot
        .error
        .as_ref()
        .unwrap()
        .message
        .contains("secret"));
    assert!(job.snapshot.restored && job.snapshot.deferred && !job.started);
    assert!(restored_bus.find_duplicate(&request).is_some());
}

#[test]
fn completed_records_survive_reconciliation_missing_files_and_protected_history() {
    let dir = tempfile::tempdir().unwrap();
    let mut queues = QueueRuntime::new(false);
    let bus = EventBus::new(Arc::new(|| {}));
    let mut jobs = BTreeMap::new();
    let id = admit(
        request(dir.path(), "completed.bin"),
        &mut jobs,
        &mut VecDeque::new(),
        &mut 1,
        &bus,
        &queues,
        false,
    )
    .unwrap();
    let job = jobs.get_mut(&id).unwrap();
    job.snapshot.state = JobState::Completed;
    job.snapshot.received = 987;
    job.snapshot.total = Some(987);
    publish_job(job, &bus);
    let entry = job.snapshot.entry_id.unwrap();
    queues
        .store
        .update(|state| {
            state.entries.get_mut(&entry).unwrap().authenticated = true;
            Ok(())
        })
        .unwrap();
    queues.reconcile(&mut jobs, &bus);
    assert!(jobs[&id].snapshot.entry_id.is_some());
    let (_queues, restored, _) = restore(queues.store.clone());
    let snapshot = &restored.values().next().unwrap().snapshot;
    assert_eq!(snapshot.state, JobState::Completed);
    assert_eq!(snapshot.received, 987);
    assert_eq!(snapshot.total, Some(987));
    assert_eq!(snapshot.readiness, EntryReadiness::Ready);
    assert!(!snapshot.destination.exists());
}

#[test]
fn protected_and_quarantined_entries_remain_held_without_credentials() {
    let dir = tempfile::tempdir().unwrap();
    let queues = QueueRuntime::new(false);
    let bus = EventBus::new(Arc::new(|| {}));
    let mut jobs = BTreeMap::new();
    let id = admit(
        request(dir.path(), "held.bin"),
        &mut jobs,
        &mut VecDeque::new(),
        &mut 1,
        &bus,
        &queues,
        true,
    )
    .unwrap();
    let entry = jobs[&id].snapshot.entry_id.unwrap();
    queues
        .store
        .update(|state| {
            let entry = state.entries.get_mut(&entry).unwrap();
            entry.refresh_needs_capture = true;
            entry.authenticated = true;
            Ok(())
        })
        .unwrap();
    let (_queues, restored, _) = restore(queues.store.clone());
    let job = restored.values().next().unwrap();
    assert_eq!(job.snapshot.state, JobState::Waiting);
    assert_eq!(job.snapshot.readiness, EntryReadiness::RecoveryRequired);
    assert!(job.snapshot.deferred && !job.started);
    queues.finish_admission(entry).unwrap();
    let (_queues, restored, _) = restore(queues.store.clone());
    let job = restored.values().next().unwrap();
    assert_eq!(job.snapshot.readiness, EntryReadiness::RestartRequired);
    assert!(job.request.browser_context_is_empty());
    assert!(job.request.authorization.is_none());
}

#[tokio::test]
async fn retry_keeps_entry_and_creation_date_and_deletion_preserves_destination() {
    let dir = tempfile::tempdir().unwrap();
    let queues = QueueRuntime::new(false);
    let bus = EventBus::new(Arc::new(|| {}));
    let mut jobs = BTreeMap::new();
    let id = admit(
        request(dir.path(), "retry.bin"),
        &mut jobs,
        &mut VecDeque::new(),
        &mut 1,
        &bus,
        &queues,
        false,
    )
    .unwrap();
    let job = jobs.get_mut(&id).unwrap();
    job.snapshot.state = JobState::Failed;
    job.snapshot.error = Some(JobError::new(ErrorKind::Http, "HTTP 503"));
    publish_job(job, &bus);
    let entry = job.snapshot.entry_id.unwrap();
    let created = job.snapshot.created_at;
    assert!(restart_job(job, &bus));
    assert_eq!(job.snapshot.entry_id, Some(entry));
    assert_eq!(job.snapshot.created_at, created);
    assert_eq!(job.snapshot.finished_at, None);
    let history = queues.store.history(entry).unwrap();
    assert_eq!(history.outcome, None);
    assert!(history.error.is_none());
    std::fs::write(&job.snapshot.destination, b"completed data").unwrap();
    queues.remove_entries(&[entry]).unwrap();
    assert_eq!(
        std::fs::read(&job.snapshot.destination).unwrap(),
        b"completed data"
    );
    let (_queues, restored, _) = restore(queues.store.clone());
    assert!(restored.is_empty());
}

#[test]
fn segmented_failure_keeps_checkpoint_and_queue_membership() {
    let dir = tempfile::tempdir().unwrap();
    let mut queues = QueueRuntime::new(false);
    let bus = EventBus::new(Arc::new(|| {}));
    let mut jobs = BTreeMap::new();
    let mut req = request(dir.path(), "segmented.bin");
    req.deferred = true;
    let id = admit(
        req,
        &mut jobs,
        &mut VecDeque::new(),
        &mut 1,
        &bus,
        &queues,
        false,
    )
    .unwrap();
    let job = jobs.get_mut(&id).unwrap();
    let partial = dir.path().join("partial");
    std::fs::write(&partial, [0u8; 64]).unwrap();
    let file = std::fs::File::open(&partial).unwrap();
    let hash = ring::digest::digest(&ring::digest::SHA256, &[0u8; 64])
        .as_ref()
        .to_vec();
    let checkpoint = crate::storage::Checkpoint {
        refresh_pending: false,
        creation_time: None,
        partial: partial.clone(),
        identity: crate::output::identify_destination(&file).unwrap(),
        durable: 64,
        prefix_hash: hash.clone(),
        bytes_hash: hash.clone(),
        total: Some(128),
        response_url: Some("https://example.com/download".into()),
        etag: Some("\"v1\"".into()),
        last_modified: None,
        cleanup_pending: vec![],
        segments: vec![crate::storage::SegmentCheckpoint {
            start: 64,
            end: 127,
            saved: 0,
            partial: dir.path().join("segment"),
            identity: crate::output::identify_destination(&file).unwrap(),
            prefix_hash: hash.clone(),
            bytes_hash: hash,
        }],
    };
    job.request
        .recovery
        .as_ref()
        .unwrap()
        .save(checkpoint.clone())
        .unwrap();
    job.snapshot.state = JobState::Failed;
    job.snapshot.received = 64;
    job.snapshot.total = Some(128);
    job.snapshot.error = Some(JobError::new(ErrorKind::Network, "Disconnected"));
    publish_job(job, &bus);
    queues.reconcile(&mut jobs, &bus);
    let (_queues, restored, _) = restore(queues.store.clone());
    let job = restored.values().next().unwrap();
    assert_eq!(job.snapshot.state, JobState::Failed);
    assert_eq!(job.snapshot.queue_id, Some(QueueId::MAIN));
    assert_eq!(job.snapshot.received, 64);
    assert!(job.request.recovery.as_ref().unwrap().checkpoint() == Some(checkpoint));
    assert!(partial.exists());
}

#[test]
fn media_history_keeps_selection_and_actual_final_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let queues = QueueRuntime::new(false);
    let bus = EventBus::new(Arc::new(|| {}));
    let mut jobs = BTreeMap::new();
    let mut req = request(dir.path(), "video.mp4");
    req.media = Some(crate::media::MediaSelection {
        identity: "Generic:public".into(),
        format_id: "720".into(),
        container: "mp4".into(),
        quality: "720p".into(),
        audio: "aac".into(),
        kind: rusty_browser_bridge::protocol::MediaKind::Hls,
    });
    let id = admit(
        req,
        &mut jobs,
        &mut VecDeque::new(),
        &mut 1,
        &bus,
        &queues,
        false,
    )
    .unwrap();
    let job = jobs.get_mut(&id).unwrap();
    job.request
        .recovery
        .as_ref()
        .unwrap()
        .finish(12345)
        .unwrap();
    job.snapshot.state = JobState::Completed;
    job.snapshot.received = 12345;
    job.snapshot.total = Some(12345);
    publish_job(job, &bus);
    let (_queues, restored, _) = restore(queues.store.clone());
    let job = restored.values().next().unwrap();
    assert_eq!(job.snapshot.received, 12345);
    assert_eq!(job.request.media.as_ref().unwrap().quality, "720p");
    assert_eq!(
        job.request
            .recovery
            .as_ref()
            .unwrap()
            .store
            .history(job.snapshot.entry_id.unwrap())
            .unwrap()
            .outcome,
        Some(HistoryOutcome::Completed)
    );
}

#[test]
fn cancel_then_retry_reuses_original_history_entry_without_restoring_cancellations() {
    let dir = tempfile::tempdir().unwrap();
    let mut queues = QueueRuntime::new(false);
    let bus = EventBus::new(Arc::new(|| {}));
    let mut jobs = BTreeMap::new();
    let id = admit(
        request(dir.path(), "cancel.bin"),
        &mut jobs,
        &mut VecDeque::new(),
        &mut 1,
        &bus,
        &queues,
        false,
    )
    .unwrap();
    let original = jobs[&id].snapshot.entry_id.unwrap();
    let created = jobs[&id].snapshot.created_at;
    cancel_job(jobs.get_mut(&id).unwrap(), &bus);
    queues.reconcile(&mut jobs, &bus);
    assert!(queues.store.snapshot().entries.is_empty());
    assert!(restore(queues.store.clone()).1.is_empty());
    let job = jobs.get_mut(&id).unwrap();
    assert!(restart_job(job, &bus));
    assert_eq!(job.snapshot.entry_id, Some(original));
    assert_eq!(job.snapshot.created_at, created);
    job.snapshot.state = JobState::Completed;
    job.snapshot.received = 19;
    job.snapshot.total = Some(19);
    publish_job(job, &bus);
    let (_, restored, _) = restore(queues.store.clone());
    let restored = &restored.values().next().unwrap().snapshot;
    assert_eq!(restored.entry_id, Some(original));
    assert_eq!(restored.state, JobState::Completed);
    assert_eq!(restored.created_at, created);
}

#[test]
fn late_running_progress_cannot_erase_worker_completion() {
    let dir = tempfile::tempdir().unwrap();
    let queues = QueueRuntime::new(false);
    let bus = EventBus::new(Arc::new(|| {}));
    let mut jobs = BTreeMap::new();
    let id = admit(
        request(dir.path(), "race.bin"),
        &mut jobs,
        &mut VecDeque::new(),
        &mut 1,
        &bus,
        &queues,
        false,
    )
    .unwrap();
    let job = jobs.get_mut(&id).unwrap();
    job.snapshot.state = JobState::Running;
    job.snapshot.last_try = Some(SystemTime::now());
    publish_job(job, &bus);
    let recovery = job.request.recovery.as_ref().unwrap().clone();
    recovery.finish(91).unwrap();
    publish_job(job, &bus);
    assert_eq!(
        recovery.store.history(recovery.entry).unwrap().outcome,
        Some(HistoryOutcome::Completed)
    );
    assert_eq!(recovery.store.history(recovery.entry).unwrap().received, 91);
}

#[test]
fn recovered_finalization_uses_new_completion_bytes_and_dates() {
    let dir = tempfile::tempdir().unwrap();
    let queues = QueueRuntime::new(false);
    let bus = EventBus::new(Arc::new(|| {}));
    let mut jobs = BTreeMap::new();
    let id = admit(
        request(dir.path(), "published.bin"),
        &mut jobs,
        &mut VecDeque::new(),
        &mut 1,
        &bus,
        &queues,
        false,
    )
    .unwrap();
    let job = &jobs[&id];
    let bytes = [7u8; 66];
    std::fs::write(&job.snapshot.destination, bytes).unwrap();
    let file = std::fs::File::open(&job.snapshot.destination).unwrap();
    let hash = ring::digest::digest(&ring::digest::SHA256, &bytes)
        .as_ref()
        .to_vec();
    job.request
        .recovery
        .as_ref()
        .unwrap()
        .journal(
            crate::storage::Checkpoint {
                refresh_pending: false,
                creation_time: None,
                partial: dir.path().join("moved.partial"),
                identity: crate::output::identify_destination(&file).unwrap(),
                durable: 66,
                prefix_hash: hash.clone(),
                bytes_hash: hash,
                total: Some(66),
                response_url: Some("https://example.com/download".into()),
                etag: Some("\"v1\"".into()),
                last_modified: None,
                segments: vec![],
                cleanup_pending: vec![],
            },
            None,
        )
        .unwrap();
    drop(file);
    let (_, restored, _) = restore(queues.store.clone());
    let job = restored.values().next().unwrap();
    assert_eq!(job.snapshot.state, JobState::Completed);
    assert_eq!(job.snapshot.received, 66);
    assert_eq!(job.snapshot.total, Some(66));
    assert!(job.snapshot.finished_at.is_some());
    assert!(job.snapshot.restored);
}

fn terminal(engine: &EngineHandle) -> JobSnapshot {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(snapshot) = engine
            .drain_events()
            .into_iter()
            .find_map(|event| match event {
                Event::JobChanged(snapshot) if snapshot.state.is_terminal() => Some(snapshot),
                _ => None,
            })
        {
            return snapshot;
        }
        assert!(Instant::now() < deadline, "Missing terminal event");
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn atomic_shutdown_flushes_recovered_storage_without_reclassifying_published_success() {
    let dir = tempfile::tempdir().unwrap();
    let captured: CapturedStore = Arc::new(Mutex::new(None));
    let engine = spawn_engine_with_executor(
        Arc::new(|| {}),
        Arc::new(HistoryExecutor {
            captured: captured.clone(),
            fail: false,
            block_final_save: true,
        }),
    );
    engine
        .try_send(Command::Submit(request(dir.path(), "shutdown.bin")))
        .unwrap();
    let snapshot = terminal(&engine);
    assert_eq!(snapshot.state, JobState::Completed);
    let (store, entry) = captured.lock().unwrap().clone().unwrap();
    assert!(store.has_pending_history());
    store.set_write_failure_for_test(false);
    engine.client().request_shutdown();
    engine.join().unwrap();
    assert!(!store.has_pending_history());
    assert_eq!(
        store.reloaded_for_test().history(entry).unwrap().outcome,
        Some(HistoryOutcome::Completed)
    );
}

#[test]
fn delete_completed_failure_does_not_publish_removal_or_lose_history() {
    let dir = tempfile::tempdir().unwrap();
    let captured: CapturedStore = Arc::new(Mutex::new(None));
    let engine = spawn_engine_with_executor(
        Arc::new(|| {}),
        Arc::new(HistoryExecutor {
            captured: captured.clone(),
            fail: false,
            block_final_save: false,
        }),
    );
    engine
        .try_send(Command::Submit(request(dir.path(), "delete.bin")))
        .unwrap();
    let snapshot = terminal(&engine);
    let (store, entry) = captured.lock().unwrap().clone().unwrap();
    store.set_write_failure_for_test(true);
    engine.try_send(Command::RemoveCompleted).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let events = engine.drain_events();
        assert!(!events
            .iter()
            .any(|event| matches!(event, Event::JobRemoved(_))));
        if events
            .iter()
            .any(|event| matches!(event, Event::CommandRejected { .. }))
        {
            break;
        }
        assert!(Instant::now() < deadline, "Missing deletion rejection");
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(store.history(entry).is_some());
    assert!(engine
        .find_duplicate(&request(dir.path(), "delete.bin"))
        .is_some_and(|job| job.id == snapshot.id));
    store.set_write_failure_for_test(false);
    engine.client().request_shutdown();
    engine.join().unwrap();
}
