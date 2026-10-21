use super::*;
use crate::storage::{Recovery, Store, StoredEntry, StoredQueue};
use std::collections::BTreeSet;
#[path = "service_scheduling.rs"]
mod scheduling;
pub(super) struct QueueRuntime {
    pub store: Arc<Store>,
    running: BTreeSet<QueueId>,
    stopping: BTreeSet<QueueId>,
    errors: BTreeMap<QueueId, String>,
    observed_failures: BTreeSet<JobId>,
    last: Vec<QueueSnapshot>,
    startup_seen: BTreeSet<QueueId>,
    stops: BTreeMap<QueueId, crate::schedule::StopDeadline>,
    stopped_minute: BTreeMap<QueueId, LocalStamp>,
    attempted_start: BTreeMap<QueueId, (u64, LocalStamp)>,
}
pub(super) fn handles(c: &Command) -> bool {
    matches!(
        c,
        Command::CreateQueue(_)
            | Command::CreateQueueForEditor { .. }
            | Command::ConfigureQueue { .. }
            | Command::RenameQueue(..)
            | Command::DeleteQueue(_)
            | Command::StartQueue(_)
            | Command::StopQueue(_)
            | Command::QueueParallelism(..)
            | Command::AssignQueue(..)
            | Command::MoveQueueEntry(..)
            | Command::SupplyCredentials(..)
            | Command::ConfirmRestart(_)
            | Command::SetCategory(..)
    )
}
impl QueueRuntime {
    pub fn new(durable: bool) -> Self {
        Self {
            store: Store::open(durable),
            running: BTreeSet::new(),
            stopping: BTreeSet::new(),
            errors: BTreeMap::new(),
            observed_failures: BTreeSet::new(),
            last: vec![],
            startup_seen: BTreeSet::new(),
            stops: BTreeMap::new(),
            stopped_minute: BTreeMap::new(),
            attempted_start: BTreeMap::new(),
        }
    }
    pub fn admit(&self, request: &mut DownloadRequest, pending: bool) -> Result<EntryId, String> {
        let id = self.store.update(|s| {
            let id = EntryId(s.next_entry);
            s.next_entry += 1;
            let mut entry = stored(id, request);
            entry.admission_pending = pending;
            s.entries.insert(id, entry);
            if request.deferred {
                s.queues
                    .get_mut(&request.queue.unwrap_or(QueueId::MAIN))
                    .ok_or("Main queue is unavailable.")?
                    .entries
                    .push(id);
            }
            Ok(id)
        })?;
        request.recovery = Some(Recovery {
            store: self.store.clone(),
            entry: id,
        });
        Ok(id)
    }
    pub fn finish_admission(&self, entry: EntryId) -> Result<(), String> {
        self.store.update(|s| {
            s.entries
                .get_mut(&entry)
                .ok_or("Admission record is missing.")?
                .admission_pending = false;
            Ok(())
        })
    }
    pub fn rollback_admission(&self, entry: EntryId) -> Result<(), String> {
        self.store.update(|s| {
            s.entries.remove(&entry);
            for queue in s.queues.values_mut() {
                queue.entries.retain(|e| *e != entry);
            }
            Ok(())
        })
    }
    pub fn restore(&mut self, jobs: &mut BTreeMap<JobId, Job>, next: &mut u64, bus: &EventBus) {
        if let Some(problem) = self.store.problem() {
            reject(bus, problem);
        }
        let state = self.store.snapshot();
        for entry in state.entries.values() {
            let mut request = match validate_request(
                &entry.url,
                entry.destination.clone(),
                ReplacementPolicy::Deny,
            ) {
                Ok(r) => r,
                Err(_) => {
                    reject(bus,"A stored download has an invalid address or path. Queue storage was preserved.");
                    continue;
                }
            };
            request.options = entry.options.clone();
            request.deferred = true;
            request.category = entry.category.clone();
            request.description = entry.description.clone();
            request.source_page = entry.source_page.clone();
            request.media = entry.media.clone();
            request.recovery = Some(Recovery {
                store: self.store.clone(),
                entry: entry.id,
            });
            let mut retained_files = entry.retained_files.clone();
            if request.media.is_some() {
                if let Ok(stage) = crate::media::stage_directory(&request) {
                    let occupied = std::fs::symlink_metadata(&stage).ok().is_some_and(|info| {
                        info.is_dir()
                            && !info.file_type().is_symlink()
                            && std::fs::read_dir(&stage)
                                .ok()
                                .is_some_and(|mut files| files.next().is_some())
                    });
                    if occupied && !retained_files.contains(&stage) {
                        retained_files.push(stage.clone());
                        if self
                            .store
                            .update(|saved| {
                                let record = saved
                                    .entries
                                    .get_mut(&entry.id)
                                    .ok_or("Media recovery entry is missing.")?;
                                if !record.retained_files.contains(&stage) {
                                    record.retained_files.push(stage);
                                }
                                Ok(())
                            })
                            .is_err()
                        {
                            reject(bus, "The interrupted media staging path could not be saved for recovery.");
                        }
                    }
                }
            }
            let mut readiness = if entry.authenticated {
                EntryReadiness::CredentialsRequired
            } else {
                EntryReadiness::Ready
            };
            let mut error = None;
            if entry.admission_pending {
                readiness = EntryReadiness::RecoveryRequired;
                error=Some(JobError::new(ErrorKind::Storage,"An unacknowledged admission was preserved after a storage interruption. The browser may still own its download. Remove this held entry before adding it again."));
            }
            let mut completed = entry.completed;
            if entry.staging.is_some() {
                if let Err(e) = crate::output::recover_staging(
                    entry,
                    &request.recovery.clone().expect("recovery"),
                ) {
                    readiness = EntryReadiness::RecoveryRequired;
                    error = Some(e);
                }
            }
            if entry.options.confirmation_required && readiness == EntryReadiness::Ready {
                readiness = EntryReadiness::ConfirmationRequired;
            }
            if entry.journal.is_some() {
                match crate::output::recover_finalization(
                    entry,
                    &request.recovery.clone().expect("recovery"),
                ) {
                    Ok(done) => completed = done,
                    Err(e) => {
                        readiness = EntryReadiness::RecoveryRequired;
                        error = Some(e);
                    }
                }
            }
            if !completed
                && entry.media.is_none()
                && entry.checkpoint.as_ref().is_some_and(|c| {
                    c.received() > 0
                        && ((c.etag.is_none() && c.last_modified.is_none())
                            || c.total.is_none()
                            || c.response_url.is_none())
                })
                && readiness == EntryReadiness::Ready
            {
                readiness = EntryReadiness::RestartRequired;
            }
            if !completed
                && entry.refresh_needs_capture
                && readiness != EntryReadiness::RecoveryRequired
            {
                readiness = EntryReadiness::RestartRequired;
                error = Some(JobError::new(ErrorKind::RestartRequired,
                    "This protected download needs a fresh browser capture after restart. Use Refresh download address to keep the saved bytes."));
            }
            let id = JobId(*next);
            *next += 1;
            let queue_id = state
                .queues
                .values()
                .find(|q| q.entries.contains(&entry.id))
                .map(|q| q.id);
            request.queue = queue_id;
            let received = entry
                .completed_bytes
                .unwrap_or_else(|| entry.checkpoint.as_ref().map_or(0, |c| c.received()));
            let snapshot = JobSnapshot {
                id,
                entry_id: Some(entry.id),
                restored: true,
                recovery_paths: retained_files,
                queue_id,
                readiness,
                filename: entry
                    .destination
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into(),
                source_host: request.source_host().into(),
                display_url: request.display_url(),
                destination: entry.destination.clone(),
                state: if completed {
                    JobState::Completed
                } else {
                    JobState::Waiting
                },
                received,
                total: if completed {
                    Some(received)
                } else {
                    entry.checkpoint.as_ref().and_then(|c| c.total)
                },
                resume_capable: entry.media.is_some()
                    || entry
                        .checkpoint
                        .as_ref()
                        .is_some_and(|c| c.etag.is_some() || c.last_modified.is_some()),
                refreshable: entry.media.is_none()
                    && entry.checkpoint.as_ref().is_some_and(|c| {
                        c.received() > 0 && c.total.is_some_and(|total| c.received() < total)
                    }),
                speed_limit: None,
                bytes_per_second: 0,
                eta_seconds: None,
                last_try: None,
                error,
                deferred: true,
                category: entry.category.clone(),
                description: entry.description.clone(),
            };
            bus.register_request(id, &request);
            bus.publish(Event::JobChanged(snapshot.clone()));
            jobs.insert(
                id,
                Job {
                    request,
                    snapshot,
                    cancel: TransferControl::new(CancellationToken::new()),
                    remove: false,
                    speed_meter: SpeedMeter::new(Instant::now(), received),
                    started: false,
                    restart: false,
                    resume_after_pause: false,
                    user_held: false,
                },
            );
        }
        self.publish(jobs, bus);
    }
    pub fn reconcile(&mut self, jobs: &mut BTreeMap<JobId, Job>, bus: &EventBus) {
        let ephemeral: Vec<_> = self
            .store
            .snapshot()
            .entries
            .values()
            .filter(|e| e.ephemeral)
            .filter_map(|e| {
                jobs.iter()
                    .find(|(_, j)| {
                        j.snapshot.entry_id == Some(e.id)
                            && matches!(j.snapshot.state, JobState::Completed | JobState::Cancelled)
                            && j.snapshot.recovery_paths.is_empty()
                            && j.snapshot.error.is_none()
                    })
                    .map(|(id, _)| (*id, e.id))
            })
            .collect();
        if !ephemeral.is_empty()
            && self
                .store
                .update(|s| {
                    for (_, entry) in &ephemeral {
                        s.entries.remove(entry);
                    }
                    Ok(())
                })
                .is_ok()
        {
            for (id, _) in ephemeral {
                if let Some(job) = jobs.get_mut(&id) {
                    job.request.recovery = None;
                    job.snapshot.entry_id = None;
                    bus.publish(Event::JobChanged(job.snapshot.clone()));
                }
            }
        }
        let state = self.store.snapshot();
        let removed: Vec<_> = state
            .entries
            .keys()
            .filter(|id| !jobs.values().any(|j| j.snapshot.entry_id == Some(**id)))
            .copied()
            .collect();
        if removed.is_empty() {
            return;
        }
        if let Err(error) = self.store.update(|s| {
            for id in removed {
                s.entries.remove(&id);
                for q in s.queues.values_mut() {
                    q.entries.retain(|e| *e != id);
                }
            }
            Ok(())
        }) {
            reject(bus, &error);
        }
    }
    pub fn slot_available(&self, job: &Job, jobs: &BTreeMap<JobId, Job>) -> bool {
        let Some(id) = job.snapshot.queue_id else {
            return true;
        };
        let limit = self
            .store
            .snapshot()
            .queues
            .get(&id)
            .map_or(1, |q| q.parallelism) as usize;
        jobs.values()
            .filter(|j| {
                j.snapshot.queue_id == Some(id)
                    && matches!(
                        j.snapshot.state,
                        JobState::Running | JobState::Pausing | JobState::Cancelling
                    )
            })
            .count()
            < limit
    }
    fn hold(
        &mut self,
        id: QueueId,
        jobs: &mut BTreeMap<JobId, Job>,
        waiting: &mut VecDeque<JobId>,
        bus: &EventBus,
    ) {
        self.running.remove(&id);
        self.stops.remove(&id);
        self.stopping.insert(id);
        for job in jobs
            .values_mut()
            .filter(|j| j.snapshot.queue_id == Some(id))
        {
            job.resume_after_pause = false;
            waiting.retain(|i| *i != job.snapshot.id);
            match job.snapshot.state {
                JobState::Running if job.cancel.try_pause() => {
                    job.snapshot.state = JobState::Pausing
                }
                JobState::Waiting if job.started => job.snapshot.state = JobState::Paused,
                JobState::Waiting => {}
                _ => continue,
            }
            job.snapshot.deferred = true;
            job.request.deferred = true;
            bus.publish(Event::JobChanged(job.snapshot.clone()));
        }
    }
    pub fn pump(
        &mut self,
        jobs: &mut BTreeMap<JobId, Job>,
        waiting: &mut VecDeque<JobId>,
        bus: &EventBus,
    ) {
        let state = self.store.snapshot();
        let failed: Vec<_> = jobs
            .values()
            .filter(|j| {
                j.snapshot
                    .error
                    .as_ref()
                    .is_some_and(|e| e.kind == ErrorKind::Storage)
            })
            .filter_map(|j| {
                j.snapshot.queue_id.map(|q| {
                    (
                        j.snapshot.id,
                        q,
                        j.snapshot.error.as_ref().unwrap().message.clone(),
                    )
                })
            })
            .collect();
        self.observed_failures
            .retain(|id| jobs.get(id).is_some_and(|j| j.snapshot.error.is_some()));
        for (job, q, e) in failed {
            if self.observed_failures.insert(job) {
                self.errors.insert(q, e);
                self.hold(q, jobs, waiting, bus);
            }
        }
        self.stopping.retain(|q| {
            jobs.values().any(|j| {
                j.snapshot.queue_id == Some(*q)
                    && matches!(
                        j.snapshot.state,
                        JobState::Running | JobState::Pausing | JobState::Cancelling
                    )
            })
        });
        for id in &self.running {
            let Some(q) = state.queues.get(id) else {
                continue;
            };
            let mut count = jobs
                .values()
                .filter(|j| {
                    j.snapshot.queue_id == Some(*id)
                        && (matches!(
                            j.snapshot.state,
                            JobState::Running | JobState::Pausing | JobState::Cancelling
                        ) || waiting.contains(&j.snapshot.id))
                })
                .count();
            for entry in &q.entries {
                if count >= q.parallelism as usize {
                    break;
                }
                let Some(job) = jobs
                    .values_mut()
                    .find(|j| j.snapshot.entry_id == Some(*entry))
                else {
                    continue;
                };
                if job.user_held
                    || job.snapshot.readiness != EntryReadiness::Ready
                    || !matches!(job.snapshot.state, JobState::Waiting | JobState::Paused)
                    || waiting.contains(&job.snapshot.id)
                {
                    continue;
                }
                job.snapshot.state = JobState::Waiting;
                job.snapshot.deferred = false;
                job.request.deferred = false;
                waiting.push_back(job.snapshot.id);
                count += 1;
                bus.publish(Event::JobChanged(job.snapshot.clone()));
            }
        }
    }
    pub fn publish(&mut self, jobs: &BTreeMap<JobId, Job>, bus: &EventBus) {
        let snapshots: Vec<_> = self
            .store
            .snapshot()
            .queues
            .values()
            .map(|q| QueueSnapshot {
                id: q.id,
                name: q.name.clone(),
                parallelism: q.parallelism,
                revision: q.revision,
                schedule: q.schedule.config.clone(),
                schedule_status: ScheduleStatus {
                    next_start: LocalStamp::now()
                        .and_then(|now| q.schedule.config.next_start(now, q.schedule.last_fired)),
                    stop_at: self.stops.get(&q.id).map(|stop| stop.local),
                },
                running: self.running.contains(&q.id),
                stopping: self.stopping.contains(&q.id),
                entries: q.entries.clone(),
                completed: jobs
                    .values()
                    .filter(|j| {
                        j.snapshot.queue_id == Some(q.id) && j.snapshot.state == JobState::Completed
                    })
                    .count(),
                active: jobs
                    .values()
                    .filter(|j| {
                        j.snapshot.queue_id == Some(q.id)
                            && matches!(j.snapshot.state, JobState::Running | JobState::Pausing)
                    })
                    .count(),
                error: self.errors.get(&q.id).cloned(),
            })
            .collect();
        if snapshots != self.last {
            self.last = snapshots.clone();
            bus.publish(Event::QueuesChanged(snapshots));
        }
    }
    pub async fn command(
        &mut self,
        c: Command,
        jobs: &mut BTreeMap<JobId, Job>,
        waiting: &mut VecDeque<JobId>,
        bus: &EventBus,
    ) {
        let result: Result<(), String> = async {
            match c {
                Command::CreateQueue(name) => {self.create_queue(name)?;},
                Command::CreateQueueForEditor{request,name}=>{
                    if !bus.queue_reply_available(){return Err("Too many queue commands await acknowledgement.".into());}
                    let result=self.create_queue(name);bus.publish(Event::QueueCreated{request,result});
                },
                Command::ConfigureQueue{request,id,settings}=>{
                    if !bus.queue_reply_available(){return Err("Too many queue commands await acknowledgement.".into());}
                    let result=self.configure_queue(id,settings);
                    if let Err(error)=&result{if error.kind==ErrorKind::Storage{self.hold(id,jobs,waiting,bus);self.errors.insert(id,error.message.clone());}}
                    bus.publish(Event::QueueConfigured{request,result:result.map_err(|error|error.message)});
                },
                Command::RenameQueue(id, name) => {
                    if id == QueueId::MAIN {
                        return Err("The Main download queue is permanent.".into());
                    }
                    let name = name.trim();
                    check_name(name)?;
                    self.store.update(|s| {
                        if s.queues
                            .values()
                            .any(|q| q.id != id && q.name.eq_ignore_ascii_case(name))
                        {
                            return Err("A queue already has that name.".into());
                        }
                        s.queues.get_mut(&id).ok_or("Queue no longer exists.")?.name = name.into();
                        Ok(())
                    })?;
                }
                Command::DeleteQueue(id) => {
                    if id == QueueId::MAIN {
                        return Err("The Main download queue is permanent.".into());
                    }
                    self.store.update(|s| {
                        if !s
                            .queues
                            .get(&id)
                            .ok_or("Queue no longer exists.")?
                            .entries
                            .is_empty()
                        {
                            return Err("Only empty queues can be deleted.".into());
                        }
                        s.queues.remove(&id);
                        Ok(())
                    })?;
                    self.running.remove(&id);
                    self.stops.remove(&id);
                    self.stopped_minute.remove(&id);
                    self.attempted_start.remove(&id);
                }
                Command::StartQueue(id) => {if let Err(error)=self.start_queue(id,jobs){self.hold(id,jobs,waiting,bus);self.errors.insert(id,error.clone());return Err(error);}},
                Command::StopQueue(id) => self.hold(id, jobs, waiting, bus),
                Command::QueueParallelism(id, n) => {
                    if !(1..=2).contains(&n) {
                        return Err("Parallelism must be one or two.".into());
                    }
                    self.store.update(|s| {
                        let queue=s.queues.get_mut(&id).ok_or("Queue no longer exists.")?;queue.parallelism=n;queue.revision=queue.revision.saturating_add(1);
                        Ok(())
                    })?;
                }
                Command::AssignQueue(id, target) => {
                    let job = jobs.get_mut(&id).ok_or("Download no longer exists.")?;
                    if matches!(
                        job.snapshot.state,
                        JobState::Running | JobState::Pausing | JobState::Cancelling
                    ) {
                        return Err("Pause the download and wait for acknowledgement before changing its queue.".into());
                    }
                    let needs_checkpoint=job.started && !job.snapshot.state.is_terminal() && job.request.recovery.is_none();
                    let entry = self.store.update(|s| {
                        if target.is_some_and(|q| !s.queues.contains_key(&q)) {
                            return Err("Queue no longer exists.".into());
                        }
                        let entry = job.snapshot.entry_id.unwrap_or_else(|| {
                            let e = EntryId(s.next_entry);
                            s.next_entry += 1;
                            e
                        });
                        let saved=s.entries.entry(entry).or_insert_with(||stored(entry,&job.request));saved.ephemeral=false;
                        if job.snapshot.state==JobState::Completed {saved.completed=true;saved.completed_bytes=Some(job.snapshot.received);}
                        for path in job.snapshot.recovery_paths.iter().cloned().chain(job.snapshot.error.iter().flat_map(|e|e.retained_partial.iter().chain(e.retained_original.iter()).cloned())) {if !saved.retained_files.contains(&path){saved.retained_files.push(path);}}
                        for q in s.queues.values_mut() {
                            q.entries.retain(|e| *e != entry);
                        }
                        if let Some(q) = target {
                            s.queues.get_mut(&q).expect("checked").entries.push(entry);
                        }
                        Ok(entry)
                    })?;
                    waiting.retain(|i| *i != id);
                    job.snapshot.entry_id = Some(entry);
                    job.snapshot.queue_id = target;
                    job.request.queue = target;
                    job.snapshot.recovery_paths=self.store.snapshot().entries.get(&entry).map(|e|e.retained_files.clone()).unwrap_or_default();
                    job.snapshot.deferred = true;
                    job.request.deferred = true;
                    job.request.recovery = Some(Recovery {
                        store: self.store.clone(),
                        entry,
                    });
                    if job.snapshot.state == JobState::Waiting && job.started {
                        job.snapshot.state = JobState::Paused;
                    }
                    job.user_held=true;
                    job.cancel.attach_recovery(job.request.recovery.clone());
                    if needs_checkpoint {
                        if let Err(error)=job.cancel.wait_checkpoint().await {
                            job.snapshot.error=Some(error.clone());
                            job.snapshot.readiness=EntryReadiness::RecoveryRequired;
                            bus.publish(Event::JobChanged(job.snapshot.clone()));
                            return Err(error.message);
                        }
                    }
                    bus.publish(Event::JobChanged(job.snapshot.clone()));
                }
                Command::MoveQueueEntry(id, up) => {
                    let job = jobs.get(&id).ok_or("Download no longer exists.")?;
                    let q = job.snapshot.queue_id.ok_or("Choose a queued download.")?;
                    let e = job.snapshot.entry_id.ok_or("Missing stored entry.")?;
                    self.store.update(|s| {
                        let entries = &mut s
                            .queues
                            .get_mut(&q)
                            .ok_or("Queue no longer exists.")?
                            .entries;
                        let i = entries
                            .iter()
                            .position(|x| *x == e)
                            .ok_or("Entry is no longer in this queue.")?;
                        let next = if up {
                            i.checked_sub(1)
                        } else {
                            (i + 1 < entries.len()).then_some(i + 1)
                        };
                        if let Some(next) = next {
                            entries.swap(i, next);
                        }
                        Ok(())
                    })?;
                    if self.running.contains(&q){
                        for job in jobs.values_mut().filter(|j|j.snapshot.queue_id==Some(q)&&j.snapshot.state==JobState::Waiting){waiting.retain(|id|*id!=job.snapshot.id);job.snapshot.deferred=true;}
                    }
                }
                Command::SupplyCredentials(id, login, password) => {
                    let job = jobs.get_mut(&id).ok_or("Download no longer exists.")?;
                    if job.snapshot.readiness != EntryReadiness::CredentialsRequired {
                        return Err("This download is not waiting for credentials.".into());
                    }
                    job.request.authorization = Some(Authorization { login, password });
                    job.snapshot.readiness = if job
                        .request
                        .recovery
                        .as_ref()
                        .and_then(Recovery::checkpoint)
                        .is_some_and(|c| c.received() > 0 && (c.etag.is_none() && c.last_modified.is_none() || c.total.is_none()))
                    {
                        EntryReadiness::RestartRequired
                    } else if job.request.options.confirmation_required {
                        EntryReadiness::ConfirmationRequired
                    } else {
                        EntryReadiness::Ready
                    };
                    bus.register_request(id, &job.request);
                    bus.publish(Event::JobChanged(job.snapshot.clone()));
                }
                Command::ConfirmRestart(id) => {
                    let job = jobs.get_mut(&id).ok_or("Download no longer exists.")?;
                    if job.snapshot.readiness != EntryReadiness::RestartRequired {
                        return Err("A restart is not required for this download.".into());
                    }
                    if let Some(recovery) = &job.request.recovery {
                        if job.request.request_context.is_empty()
                            && recovery.store.snapshot().entries.get(&recovery.entry).is_some_and(|entry| entry.refresh_needs_capture) {
                            return Err("This protected download requires a fresh browser capture. Use Refresh download address or capture it as a new download.".into());
                        }
                        crate::output::discard_checkpoint(recovery).map_err(|e| e.message)?;
                    }
                    job.request.replacement = inspect_destination(&job.request.destination)
                        .map_err(|e| e.message)?
                        .map(ReplacementPolicy::Approved)
                        .unwrap_or(ReplacementPolicy::Deny);
                    let replacement = job.request.replacement.clone();
                    reset_job(job, replacement);
                    job.snapshot.readiness = EntryReadiness::Ready;
                    job.snapshot.recovery_paths = self
                        .store
                        .snapshot()
                        .entries
                        .get(&job.snapshot.entry_id.expect("stored entry"))
                        .map(|e| e.retained_files.clone())
                        .unwrap_or_default();
                    waiting.push_back(id);
                    bus.publish(Event::JobChanged(job.snapshot.clone()));
                }
                Command::SetCategory(id, category) => {
                    let job = jobs.get_mut(&id).ok_or("Download no longer exists.")?;
                    if let Some(value) = category.as_deref() {
                        check_name(value)?;
                    }
                    if let Some(e) = job.snapshot.entry_id {
                        self.store.update(|s| {
                            s.entries
                                .get_mut(&e)
                                .ok_or("Stored entry is missing.")?
                                .category = category.clone();
                            Ok(())
                        })?;
                    }
                    job.request.category = category.clone();
                    job.snapshot.category = category;
                    bus.publish(Event::JobChanged(job.snapshot.clone()));
                }
                _ => {}
            }
            Ok(())
        }.await;
        if let Err(error) = result {
            reject(bus, &error);
        }
    }
}
fn check_name(name: &str) -> Result<(), String> {
    if name.trim().is_empty() || name.chars().count() > 160 || name.chars().any(char::is_control) {
        Err("Enter a name from 1 to 160 characters without control characters.".into())
    } else {
        Ok(())
    }
}
fn stored(id: EntryId, r: &DownloadRequest) -> StoredEntry {
    StoredEntry {
        media: r.media.clone(),
        source_page: r.source_page.clone(),
        options: r.options.clone(),
        ephemeral: !r.deferred,
        admission_pending: false,
        staging: None,
        id,
        url: r.url.as_str().into(),
        destination: r.destination.clone(),
        category: r.category.clone(),
        description: r.description.clone(),
        authenticated: r.authorization.is_some(),
        refresh_needs_capture: !r.request_context.is_empty(),
        completed: false,
        completed_bytes: None,
        retained_files: vec![],
        checkpoint: None,
        journal: None,
    }
}

impl QueueRuntime {
    pub fn snapshots(&self) -> Vec<QueueSnapshot> {
        self.last.clone()
    }
    pub fn create_for_reply(&self, name: String) -> Result<QueueId, String> {
        self.create_queue(name)
    }
    pub fn rename_categories(
        &self,
        jobs: &mut BTreeMap<JobId, Job>,
        renames: Vec<(String, String)>,
        bus: &EventBus,
    ) -> Result<CategoryRenameUndo, String> {
        if renames.len() > 128
            || renames.iter().any(|(from, to)| {
                from.trim().is_empty()
                    || to.trim().is_empty()
                    || from.chars().chain(to.chars()).any(char::is_control)
                    || to.chars().count() > 80
                    || std::iter::once("General")
                        .chain(crate::categories::BUILTIN)
                        .any(|s| s.eq_ignore_ascii_case(from) || s.eq_ignore_ascii_case(to))
            })
        {
            return Err("Invalid custom category rename.".into());
        }
        let replacement = |category: &Option<String>| {
            category
                .as_ref()
                .and_then(|c| {
                    renames
                        .iter()
                        .find(|(from, _)| c.eq_ignore_ascii_case(from))
                })
                .map(|(_, to)| Some(to.clone()))
        };
        let mut undo = CategoryRenameUndo::default();
        self.store.update(|s| {
            for e in s.entries.values_mut() {
                if let Some(after) = replacement(&e.category) {
                    undo.entries.push((e.id, e.category.clone(), after.clone()));
                    e.category = after;
                }
            }
            Ok(())
        })?;
        for (id, job) in jobs {
            if let Some(after) = replacement(&job.snapshot.category) {
                undo.jobs
                    .push((*id, job.snapshot.category.clone(), after.clone()));
                job.request.category = after.clone();
                job.snapshot.category = after;
                bus.publish(Event::JobChanged(job.snapshot.clone()));
            }
        }
        Ok(undo)
    }
    pub fn restore_categories(
        &self,
        jobs: &mut BTreeMap<JobId, Job>,
        undo: CategoryRenameUndo,
        bus: &EventBus,
    ) -> Result<(), String> {
        for (id, _, after) in &undo.jobs {
            if jobs
                .get(id)
                .is_some_and(|job| &job.snapshot.category != after)
            {
                return Err("A runtime category changed during compensation.".into());
            }
        }
        self.store.update(|s| {
            for (id, before, after) in &undo.entries {
                if let Some(e) = s.entries.get_mut(id) {
                    if &e.category != after {
                        return Err(
                            "A category changed during compensation; recovery is required.".into(),
                        );
                    }
                    e.category = before.clone();
                }
            }
            Ok(())
        })?;
        for (id, before, after) in undo.jobs {
            if let Some(job) = jobs.get_mut(&id) {
                if job.snapshot.category != after {
                    return Err("A runtime category changed during compensation.".into());
                }
                job.request.category = before.clone();
                job.snapshot.category = before;
                bus.publish(Event::JobChanged(job.snapshot.clone()));
            }
        }
        Ok(())
    }
}
