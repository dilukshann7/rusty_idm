//! Actor-owned recovery. Transfer workers finish and checkpoint before scheduling.
use super::*;

#[derive(Default)]
pub(super) struct RetryRuntime {
    used: u8,
    deadline: Option<Instant>,
    remaining: Option<Duration>,
}
impl RetryRuntime {
    fn schedule(
        &mut self,
        policy: RetryPolicy,
        now: Instant,
        wall: SystemTime,
    ) -> Option<RetryStatus> {
        if self.used >= policy.max_retries {
            return None;
        }
        self.used += 1;
        let delay = Duration::from_secs(u64::from(policy.delay_seconds));
        self.deadline = Some(now + delay);
        self.remaining = None;
        Some(RetryStatus {
            attempt: self.used,
            max_retries: policy.max_retries,
            next_retry_at: Some(wall + delay),
        })
    }
    fn due(&self, now: Instant) -> bool {
        self.deadline.is_some_and(|deadline| now >= deadline)
    }
    pub(super) fn admitted(&mut self) {
        self.deadline = None;
        self.remaining = None;
    }
    pub(super) fn reset(&mut self) {
        *self = Self::default();
    }
    pub(super) fn pause(&mut self, now: Instant, status: &mut RetryStatus) {
        if let Some(deadline) = self.deadline.take() {
            self.remaining = Some(deadline.saturating_duration_since(now));
        }
        status.next_retry_at = None;
    }
    pub(super) fn resume(&mut self, now: Instant, wall: SystemTime, status: &mut RetryStatus) {
        if let Some(remaining) = self.remaining.take() {
            self.deadline = Some(now + remaining);
            status.next_retry_at = Some(wall + remaining);
        }
    }
}

pub(super) fn clear(job: &mut Job) {
    job.retry.reset();
    job.snapshot.retry = None;
}
pub(super) fn start_now(job: &mut Job) {
    if job.snapshot.retry.is_some() {
        job.snapshot.error = None;
    }
    clear(job);
}

pub(super) fn schedule(job: &mut Job, now: Instant, wall: SystemTime) -> bool {
    if job.snapshot.state != JobState::Failed
        || job.request.media.is_some()
        || job.request.recovery.is_none()
        || job.request.options.confirmation_required
        || job.snapshot.readiness != EntryReadiness::Ready
        || job.cancel.token().is_cancelled()
        || job.cancel.preserving()
        || job.remove
        || job.restart
        || has_unmanaged_recovery(job)
        || !job
            .snapshot
            .error
            .as_ref()
            .is_some_and(|error| error.retryable)
    {
        return false;
    }
    let Some(mut status) = job
        .retry
        .schedule(job.request.options.retry_policy, now, wall)
    else {
        return false;
    };
    if let Some(checkpoint) = job
        .request
        .recovery
        .as_ref()
        .and_then(|recovery| recovery.checkpoint())
    {
        job.snapshot.received = checkpoint.received();
        job.snapshot.total = checkpoint.total;
    }
    if job.user_held {
        job.retry.pause(now, &mut status);
    }
    job.snapshot.retry = Some(status);
    job.snapshot.state = JobState::Waiting;
    job.snapshot.finished_at = None;
    job.snapshot.deferred = job.user_held;
    job.request.deferred = job.user_held;
    job.snapshot.bytes_per_second = 0;
    job.snapshot.eta_seconds = None;
    job.started = false;
    job.resume_after_pause = false;
    job.cancel = TransferControl::new(CancellationToken::new());
    job.cancel.set_speed_limit(job.snapshot.speed_limit);
    true
}

pub(super) fn ready(job: &Job, queues: &QueueRuntime, now: Instant) -> bool {
    job.snapshot.retry.is_none()
        || (!job.user_held && job.retry.due(now) && queues.retry_allowed(job))
}

pub(super) fn pump(
    jobs: &mut BTreeMap<JobId, Job>,
    waiting: &mut VecDeque<JobId>,
    queues: &QueueRuntime,
    bus: &EventBus,
    now: Instant,
    wall: SystemTime,
) {
    for job in jobs.values_mut().filter(|job| job.snapshot.retry.is_some()) {
        let blocked = job.user_held || !queues.retry_allowed(job);
        job.snapshot.deferred = blocked;
        job.request.deferred = blocked;
        if blocked {
            waiting.retain(|id| *id != job.snapshot.id);
        } else if job.retry.due(now)
            && job.snapshot.readiness == EntryReadiness::Ready
            && !waiting.contains(&job.snapshot.id)
        {
            waiting.push_back(job.snapshot.id);
        }
        // Wall-clock changes must not alter the monotonic recovery schedule.
        if let (Some(status), Some(deadline)) = (&mut job.snapshot.retry, job.retry.deadline) {
            status.next_retry_at = Some(wall + deadline.saturating_duration_since(now));
        }
        publish_job(job, bus);
    }
}

#[cfg(test)]
#[path = "service_recovery_tests.rs"]
mod tests;
