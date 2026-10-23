//! Project admissions share ordinary durable transfers and their recovery policy.
use super::*;
use crate::grabber::{TransferAction, TransferStatus};

pub(super) fn handle(
    action: TransferAction,
    jobs: &mut BTreeMap<JobId, Job>,
    waiting: &mut VecDeque<JobId>,
    next_id: &mut u64,
    bus: &EventBus,
    queues: &QueueRuntime,
) -> Result<Vec<TransferStatus>, String> {
    let (project, generation) = match &action {
        TransferAction::Admit { member, .. } => (member.project.clone(), member.generation),
        TransferAction::Query {
            project,
            generation,
        }
        | TransferAction::Hold {
            project,
            generation,
        }
        | TransferAction::Retire {
            project,
            generation,
            ..
        } => (project.clone(), *generation),
        TransferAction::Activate(member) => (member.project.clone(), member.generation),
    };
    match action {
        TransferAction::Admit {
            member,
            mut request,
        } => {
            member.validate()?;
            if member.generation == 0
                || member.item.0 == 0
                || member.allowed_hosts.is_empty()
                || !crate::grabber::host_allowed(&request.url, &member.allowed_hosts)
            {
                return Err("Invalid project download identity or host scope.".into());
            }
            let existing = jobs.values().find(|j| {
                j.request
                    .project
                    .as_ref()
                    .is_some_and(|m| m.same_item(&member))
            });
            if let Some(existing) = existing {
                if existing.request.url != request.url
                    || existing.request.destination != request.destination
                    || existing.request.project.as_ref() != Some(&member)
                {
                    return Err("The saved project download identity conflicts with this request. Rescan into a new generation.".into());
                }
            } else {
                request.project = Some(member);
                request.allow_duplicate = true;
                request.options.confirmation_required = false;
                request.deferred = false;
                request.queue = None;
                request.options.user_agent = "RustySiteGrabber/0.2.1".into();
                admit(request,jobs,waiting,next_id,bus,queues,false)
                    .map_err(|_|"Project download admission failed. Check history capacity, destination conflicts, and storage.".to_string())?;
            }
        }
        TransferAction::Hold { .. } => {
            for (id, job) in jobs
                .iter_mut()
                .filter(|(_, j)| owns(j, &project, generation))
            {
                job.user_held = true;
                job.restart = false;
                job.resume_after_pause = false;
                if let Some(status) = &mut job.snapshot.retry {
                    job.retry.pause(Instant::now(), status);
                }
                waiting.retain(|queued| queued != id);
                if job.snapshot.state == JobState::Waiting {
                    job.request.deferred = true;
                    job.snapshot.deferred = true;
                }
                if job.snapshot.state == JobState::Running && job.cancel.try_pause() {
                    job.snapshot.state = JobState::Pausing;
                }
                publish_job(job, bus);
            }
        }
        TransferAction::Activate(member) => {
            if let Some(job) = jobs.values_mut().find(|j| {
                j.request
                    .project
                    .as_ref()
                    .is_some_and(|m| m.same_item(&member))
            }) {
                if let Some(status) = &mut job.snapshot.retry {
                    job.user_held = false;
                    job.retry.resume(Instant::now(), SystemTime::now(), status);
                    publish_job(job, bus);
                } else if job.snapshot.state == JobState::Completed
                    && !job.request.destination.exists()
                {
                    if bus.terminal_pending(job.snapshot.id) {
                        return Err(
                            "Wait for the previous completion acknowledgement before retrying."
                                .into(),
                        );
                    }
                    reset_job(job, ReplacementPolicy::Deny).map_err(|e| e.message)?;
                    waiting.push_back(job.snapshot.id);
                    publish_job(job, bus);
                } else if job.snapshot.state != JobState::Completed {
                    activate_external(job,waiting,bus).map_err(|_|"This project file needs manual recovery or a restart from its download row.".to_string())?;
                }
            }
        }
        TransferAction::Retire {
            remove_internal, ..
        } => {
            for job in jobs.values_mut().filter(|j| owns(j, &project, generation)) {
                job.restart = false;
                job.resume_after_pause = false;
                job.user_held = true;
                cancel_job(job, bus);
            }
            if remove_internal {
                let ids: Vec<_> = jobs
                    .iter()
                    .filter(|(_, j)| {
                        owns(j, &project, generation)
                            && j.request.project.as_ref().is_some_and(|m| m.mirror)
                            && j.snapshot.state.is_terminal()
                    })
                    .map(|(id, j)| (*id, j.snapshot.entry_id))
                    .collect();
                let entries: Vec<_> = ids.iter().filter_map(|(_, entry)| *entry).collect();
                queues.remove_entries(&entries)?;
                for (id, _) in ids {
                    jobs.remove(&id);
                    waiting.retain(|j| *j != id);
                    bus.publish(Event::JobRemoved(id));
                }
            }
        }
        TransferAction::Query { .. } => {}
    }
    Ok(jobs
        .values()
        .filter(|j| owns(j, &project, generation))
        .map(|j| TransferStatus {
            member: j.request.project.clone().expect("project member"),
            job: j.snapshot.id,
            entry: j.snapshot.entry_id,
            state: j.snapshot.state,
            held: j.user_held || j.snapshot.deferred || j.snapshot.state == JobState::Paused,
            received: j.snapshot.received,
            destination: j.snapshot.destination.clone(),
            message: j
                .snapshot
                .error
                .as_ref()
                .map(|e| e.message.clone())
                .unwrap_or_default(),
        })
        .collect())
}
fn owns(job: &Job, project: &crate::grabber::ProjectId, generation: u64) -> bool {
    job.request
        .project
        .as_ref()
        .is_some_and(|m| &m.project == project && (generation == 0 || m.generation == generation))
}
