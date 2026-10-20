//! All queue starts, including timed starts, use the engine's existing limits.
use super::*;
impl QueueRuntime {
    pub(super) fn create_queue(&self, name: String) -> Result<QueueId, String> {
        let name = name.trim();
        check_name(name)?;
        self.store.update(|s| {
            if s.queues.values().any(|q| q.name.eq_ignore_ascii_case(name)) {
                return Err("A queue already has that name.".into());
            }
            let id = QueueId(s.next_queue);
            s.next_queue += 1;
            s.queues.insert(
                id,
                StoredQueue {
                    id,
                    name: name.into(),
                    parallelism: 1,
                    entries: vec![],
                    revision: 0,
                    schedule: Default::default(),
                },
            );
            Ok(id)
        })
    }
    pub(super) fn configure_queue(
        &mut self,
        id: QueueId,
        settings: QueueSettings,
    ) -> Result<(), JobError> {
        if !(1..=2).contains(&settings.parallelism) {
            return Err(JobError::new(
                ErrorKind::InvalidRequest,
                "Parallelism must be one or two.",
            ));
        }
        let now = LocalStamp::now().ok_or_else(|| {
            JobError::new(
                ErrorKind::InvalidRequest,
                "Cannot read the local Windows time; settings were not changed.",
            )
        })?;
        settings
            .schedule
            .validate_for_apply(now)
            .map_err(|message| JobError::new(ErrorKind::InvalidRequest, message))?;
        let prior_stop = self
            .store
            .snapshot()
            .queues
            .get(&id)
            .ok_or_else(|| JobError::new(ErrorKind::InvalidRequest, "Queue no longer exists."))?
            .schedule
            .config
            .stop_minute;
        self.store
            .update(|s| {
                let queue = s.queues.get_mut(&id).ok_or("Queue no longer exists.")?;
                if queue.schedule.config != settings.schedule {
                    queue.schedule.generation = queue.schedule.generation.saturating_add(1);
                    queue.schedule.last_fired = None;
                    queue.schedule.config = settings.schedule;
                }
                queue.parallelism = settings.parallelism;
                queue.revision = queue.revision.saturating_add(1);
                Ok(())
            })
            .map_err(crate::storage::storage_error)?;
        if self.running.contains(&id)
            && prior_stop
                != self.store.snapshot().queues[&id]
                    .schedule
                    .config
                    .stop_minute
        {
            if let Some(stop) = self.store.snapshot().queues[&id]
                .schedule
                .config
                .stop_minute
                .and_then(|minute| now.next_stop(minute))
            {
                self.stops.insert(id, stop);
            } else {
                self.stops.remove(&id);
            }
        }
        Ok(())
    }
    pub(super) fn start_queue(
        &mut self,
        id: QueueId,
        now: Option<LocalStamp>,
        jobs: &mut BTreeMap<JobId, Job>,
    ) -> Result<(), String> {
        let state = self.store.snapshot();
        let queue = state.queues.get(&id).ok_or("Queue no longer exists.")?;
        if self.stopping.contains(&id) {
            return Err("Wait for this queue to finish pausing before starting it again.".into());
        }
        // A no-op durable write detects unavailable storage before any admission.
        self.store.update(|_| Ok(()))?;
        if !self.running.contains(&id) {
            if let Some(minute) = queue.schedule.config.stop_minute {
                let stop = now
                    .and_then(|now| now.next_stop(minute))
                    .ok_or("Cannot arm the configured stop time; queue remains held.")?;
                self.stops.insert(id, stop);
            }
        }
        self.errors.remove(&id);
        self.running.insert(id);
        for job in jobs
            .values_mut()
            .filter(|j| j.snapshot.queue_id == Some(id))
        {
            job.user_held = false;
        }
        Ok(())
    }
    pub(crate) fn clock_tick(
        &mut self,
        jobs: &mut BTreeMap<JobId, Job>,
        waiting: &mut VecDeque<JobId>,
        bus: &EventBus,
    ) {
        let Some(now) = LocalStamp::now() else {
            let bounded: Vec<_> = self.stops.keys().copied().collect();
            for id in bounded {
                self.hold(id, jobs, waiting, bus);
                self.errors
                    .insert(id, "Local time is unavailable; the queue is held.".into());
            }
            return;
        };
        self.stopped_minute.retain(|_, at| *at == now);
        let overdue: Vec<_> = self
            .stops
            .iter()
            .filter(|(_, at)| **at <= now)
            .map(|(id, _)| *id)
            .collect();
        for id in overdue {
            self.hold(id, jobs, waiting, bus);
            self.stopped_minute.insert(id, now);
        }
        let state = self.store.snapshot();
        for queue in state.queues.values() {
            let startup =
                self.startup_seen.insert(queue.id) && queue.schedule.config.start_on_startup;
            let due = queue.schedule.config.due(now, queue.schedule.last_fired);
            if !startup && !due {
                continue;
            }
            let recorded = if due {
                self.store.update(|s| {
                    let saved = s
                        .queues
                        .get_mut(&queue.id)
                        .ok_or("Queue no longer exists.")?;
                    // The actor is the sole editor; checkpoints can only alter entry records.
                    if saved.schedule.generation != queue.schedule.generation {
                        return Err(
                            "Schedule changed before its occurrence could be recorded.".into()
                        );
                    }
                    saved.schedule.last_fired = Some(now);
                    if matches!(saved.schedule.config.start, StartRule::Once(_)) {
                        saved.schedule.config.enabled = false;
                    }
                    Ok(())
                })
            } else {
                self.store.update(|_| Ok(()))
            };
            if let Err(error) = recorded {
                self.hold(queue.id, jobs, waiting, bus);
                self.errors.insert(queue.id, error.clone());
                reject(
                    bus,
                    &format!("Cannot record scheduled start for {}. {error}", queue.name),
                );
                continue;
            }
            // A due stop wins this entire minute. Do not immediately resume it.
            if self.stopped_minute.contains_key(&queue.id) {
                continue;
            }
            if let Err(error) = self.start_queue(queue.id, Some(now), jobs) {
                self.hold(queue.id, jobs, waiting, bus);
                self.errors.insert(queue.id, error.clone());
                reject(
                    bus,
                    &format!("Cannot start scheduled queue {}. {error}", queue.name),
                );
            }
        }
    }
}
