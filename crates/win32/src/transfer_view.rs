use rusty_engine::{JobId, JobSnapshot, JobState};
use std::{collections::BTreeMap, time::SystemTime};

/// Tracks attempts separately from HWNDs so closing a window does not reopen it
/// on every progress update. A retry receives a new last_try from the actor.
#[derive(Default)]
pub struct AutoOpenTracker {
    attempts: BTreeMap<JobId, SystemTime>,
}
impl AutoOpenTracker {
    pub fn should_open(&mut self, job: &JobSnapshot) -> bool {
        let Some(attempt) = job.last_try else {
            return false;
        };
        if job.state == JobState::Waiting || self.attempts.get(&job.id) == Some(&attempt) {
            return false;
        }
        self.attempts.insert(job.id, attempt);
        true
    }
    pub fn forget(&mut self, id: JobId) {
        self.attempts.remove(&id);
    }
}
pub struct TransferText {
    pub title: String,
    pub url: String,
    pub status: String,
    pub size: String,
    pub downloaded: String,
    pub rate: String,
    pub time_left: String,
    pub details: String,
}
pub fn format_transfer(job: &JobSnapshot) -> TransferText {
    let percent = match job.total {
        Some(0) if job.state == JobState::Completed => Some(10_000),
        Some(total) if total > 0 => {
            Some(((job.received as u128 * 10_000) / total as u128).min(10_000))
        }
        _ => None,
    };
    let title = percent
        .map(|p| format!("{}% {}", p / 100, job.filename))
        .unwrap_or_else(|| job.filename.clone());
    let status = match job.readiness {
        rusty_engine::EntryReadiness::ConfirmationRequired => {
            "Confirm this download before starting."
        }
        rusty_engine::EntryReadiness::CredentialsRequired => "Credentials required.",
        rusty_engine::EntryReadiness::RestartRequired => "Restart required.",
        rusty_engine::EntryReadiness::RecoveryRequired => "Recovery required.",
        rusty_engine::EntryReadiness::Ready => match job.state {
            JobState::Waiting if job.deferred => "Download later",
            JobState::Waiting => "Waiting for a connection...",
            JobState::Running => "Receiving data...",
            JobState::Pausing => "Pausing download...",
            JobState::Paused => "Paused.",
            JobState::Cancelling => "Stopping download...",
            JobState::Completed => "Download complete.",
            JobState::Failed => "Download failed.",
            JobState::Cancelled => "Download cancelled.",
        },
    }
    .to_owned();
    let size = job.total.map(amount).unwrap_or_else(|| "Unknown".into());
    let downloaded = match percent {
        Some(p) => format!("{}  ( {}.{:02} % )", amount(job.received), p / 100, p % 100),
        None => amount(job.received),
    };
    let rate = format!(
        "{}/sec",
        amount(if job.state == JobState::Running {
            job.bytes_per_second
        } else {
            0
        })
    );
    let time_left = if job.state == JobState::Completed {
        "0 sec".into()
    } else if job.state == JobState::Running && job.total.is_some() {
        job.eta_seconds
            .map(duration)
            .unwrap_or_else(|| "Unknown".into())
    } else {
        "Unknown".into()
    };
    let error = job
        .error
        .as_ref()
        .map(|e| {
            format!(
                "\r\n\r\n{}{}{}",
                e.message,
                e.retained_partial
                    .as_ref()
                    .map(|p| format!("\r\nRetained partial: {}", p.display()))
                    .unwrap_or_default(),
                e.retained_original
                    .as_ref()
                    .map(|p| format!("\r\nOriginal backup: {}", p.display()))
                    .unwrap_or_default()
            )
        })
        .unwrap_or_default();
    let retained = job
        .recovery_paths
        .iter()
        .map(|p| format!("\r\nPreserved recovery file: {}", p.display()))
        .collect::<String>();
    let error = format!("{error}{retained}");
    let details = format!(
        "File: {}\r\nSave as: {}\r\nTotal: {}\r\nConnections: 1\r\nResume capability: {}\r\nSpeed limit: {}\r\n{}{}",
        job.filename,
        job.destination.display(),
        job.total
            .map(|n| format!("{n} bytes"))
            .unwrap_or_else(|| "Unknown".into()),
        if job.resume_capable { "Yes" } else { "No (resume restarts from beginning)" },
        job.speed_limit.map(|n| format!("{}/sec", amount(n))).unwrap_or_else(|| "Unlimited".into()),
        job.description,
        error
    );
    TransferText {
        title,
        url: job.display_url.clone(),
        status,
        size,
        downloaded,
        rate,
        time_left,
        details,
    }
}
fn amount(bytes: u64) -> String {
    if bytes < 1024 {
        return format!("{bytes} Bytes");
    }
    for (unit, divisor) in [("GB", 1_073_741_824u64), ("MB", 1_048_576), ("KB", 1024)] {
        if bytes >= divisor {
            return format!("{:.3} {unit}", bytes as f64 / divisor as f64);
        }
    }
    unreachable!()
}
fn duration(seconds: u64) -> String {
    if seconds < 60 {
        format!("{seconds} sec")
    } else if seconds < 3600 {
        format!("{} min {} sec", seconds / 60, seconds % 60)
    } else {
        format!("{} hr {} min", seconds / 3600, seconds / 60 % 60)
    }
}

#[cfg(test)]
#[path = "transfer_view_tests.rs"]
mod tests;
