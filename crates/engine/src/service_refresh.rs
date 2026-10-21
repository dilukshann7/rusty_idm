use super::*;
use crate::{
    refresh::{RefreshCandidate, VerifiedRefresh},
    storage::Checkpoint,
};

pub(super) fn preflight(
    jobs: &BTreeMap<JobId, Job>,
    pending: &HashMap<JobId, PendingRefresh>,
    id: JobId,
    candidate: &RefreshCandidate,
) -> Result<(Checkpoint, bool), String> {
    candidate.validate().map_err(|e| e.message)?;
    if pending.contains_key(&id) {
        return Err("A new address is already being checked for this download.".into());
    }
    let job = jobs.get(&id).ok_or("The download no longer exists.")?;
    let checkpoint = checkpoint(job)?;
    Ok((checkpoint, job.request.options.ignore_modified_time))
}

pub(super) fn checkpoint(job: &Job) -> Result<Checkpoint, String> {
    let held = job.snapshot.state == JobState::Waiting && job.snapshot.deferred && !job.started;
    if (!matches!(job.snapshot.state, JobState::Paused | JobState::Failed) && !held)
        || (job.snapshot.state == JobState::Paused && job.started)
    {
        return Err(
            "Pause the download and wait for its partial to be saved before refreshing.".into(),
        );
    }
    if job
        .snapshot
        .error
        .as_ref()
        .is_some_and(|e| e.retained_original.is_some())
    {
        return Err("Resolve the retained original file before refreshing.".into());
    }
    let recovery = job
        .request
        .recovery
        .as_ref()
        .ok_or("This download has no saved partial to refresh.")?;
    let checkpoint = recovery
        .checkpoint()
        .ok_or("This download has no saved partial to refresh.")?;
    if checkpoint.received() == 0
        || checkpoint
            .total
            .is_none_or(|total| checkpoint.received() >= total)
    {
        return Err("This download has no unfinished saved bytes to refresh.".into());
    }
    if job
        .snapshot
        .recovery_paths
        .iter()
        .any(|p| p != &checkpoint.partial)
    {
        return Err("Resolve other retained recovery files before refreshing.".into());
    }
    Ok(checkpoint)
}

pub(super) fn commit(
    job: &mut Job,
    expected: &Checkpoint,
    candidate: RefreshCandidate,
    verified: VerifiedRefresh,
) -> Result<(), String> {
    let url = candidate.validate().map_err(|e| e.message)?;
    if expected.total != Some(verified.total)
        || job
            .request
            .recovery
            .as_ref()
            .and_then(|r| r.checkpoint())
            .as_ref()
            != Some(expected)
    {
        return Err("The saved download changed while its new address was checked.".into());
    }
    let recovery = job
        .request
        .recovery
        .as_ref()
        .ok_or("The saved download disappeared.")?;
    let new_page = candidate
        .source_page
        .clone()
        .or_else(|| job.request.source_page.clone());
    recovery.store.update(|state| {
        let entry = state
            .entries
            .get_mut(&recovery.entry)
            .ok_or("The saved download disappeared.")?;
        if entry.checkpoint.as_ref() != Some(expected) {
            return Err("The saved download changed while its new address was checked.".into());
        }
        let checkpoint = entry.checkpoint.as_mut().expect("checked");
        checkpoint.response_url = Some(verified.final_url.clone());
        checkpoint.etag = verified.etag.clone();
        checkpoint.last_modified = verified.last_modified.clone();
        checkpoint.refresh_pending = true;
        entry.url = url.as_str().to_owned();
        entry.source_page = new_page.clone();
        entry.authenticated = false;
        entry.refresh_needs_capture = !candidate.request_context.is_empty();
        entry
            .retained_files
            .retain(|path| path != &checkpoint.partial);
        Ok(())
    })?;
    job.request.url = url;
    job.request.source_page = new_page;
    job.request.authorization = None;
    job.request.request_context = candidate.request_context;
    job.request.deferred = job.snapshot.queue_id.is_some();
    job.snapshot.state = JobState::Waiting;
    job.snapshot.readiness = EntryReadiness::Ready;
    job.snapshot.error = None;
    job.snapshot
        .recovery_paths
        .retain(|p| p != &expected.partial);
    job.snapshot.received = expected.received();
    job.snapshot.total = Some(verified.total);
    job.snapshot.resume_capable = true;
    job.snapshot.refreshable = true;
    job.snapshot.source_host = job.request.source_host().to_owned();
    job.snapshot.display_url = job.request.display_url();
    job.snapshot.deferred = job.snapshot.queue_id.is_some();
    job.snapshot.bytes_per_second = 0;
    job.snapshot.eta_seconds = None;
    job.snapshot.last_try = None;
    job.cancel = TransferControl::new(CancellationToken::new());
    job.cancel.set_speed_limit(job.snapshot.speed_limit);
    job.started = false;
    job.restart = false;
    job.resume_after_pause = false;
    job.user_held = false;
    Ok(())
}

pub(super) fn commit_and_queue(
    jobs: &mut BTreeMap<JobId, Job>,
    waiting: &mut VecDeque<JobId>,
    bus: &EventBus,
    id: JobId,
    expected: &Checkpoint,
    candidate: RefreshCandidate,
    verified: VerifiedRefresh,
) -> Result<(), String> {
    let job = jobs.get_mut(&id).ok_or("The download no longer exists.")?;
    if &checkpoint(job)? != expected {
        return Err("The saved download changed while its new address was checked.".into());
    }
    commit(job, expected, candidate, verified)?;
    if job.snapshot.queue_id.is_none() {
        waiting.push_back(id);
    }
    bus.publish(Event::JobChanged(job.snapshot.clone()));
    Ok(())
}
