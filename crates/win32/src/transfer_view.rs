use rusty_engine::{JobId, JobSnapshot, JobState, SegmentState};
use std::{collections::BTreeMap, time::SystemTime};

/// Tracks attempts separately from HWNDs so closing a window does not reopen it
/// on every progress update. A retry receives a new last_try from the actor.
#[derive(Default)]
pub struct AutoOpenTracker {
    attempts: BTreeMap<JobId, SystemTime>,
}
impl AutoOpenTracker {
    pub fn should_open(&mut self, job: &JobSnapshot) -> bool {
        if job.restored {
            if let Some(attempt) = job.last_try {
                self.attempts.insert(job.id, attempt);
            }
            return false;
        }
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
/// The detail button requests Pause (true), Resume (false), or stays disabled.
pub fn pause_action(job: &JobSnapshot) -> Option<bool> {
    match job.state {
        JobState::Running => Some(true),
        JobState::Paused => Some(false),
        JobState::Waiting => job
            .retry
            .as_ref()
            .map(|retry| retry.next_retry_at.is_some()),
        _ => None,
    }
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
    let status = crate::presenter::retry_status(job, SystemTime::now()).unwrap_or(status);
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
    let date = |value: Option<SystemTime>| {
        value
            .map(crate::presenter::local_time)
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "Unknown".into())
    };
    let details = format!(
        "File: {}\r\nSave as: {}\r\nTotal: {}\r\n{}\r\nResume capability: {}\r\nSpeed limit: {}\r\nAdded: {}\r\nLast Try: {}\r\nFinished: {}\r\n{}{}",
        job.filename,
        job.destination.display(),
        job.total
            .map(|n| format!("{n} bytes"))
            .unwrap_or_else(|| "Unknown".into()),
        job.transfer_details.as_ref().map(|details| format!("Connections: {} active (limit {})",
            if job.state == JobState::Running { details.active_connections } else { 0 },
            details.connection_limit)).unwrap_or_else(|| "Connections: Unavailable".into()),
        if job.resume_capable { "Yes" } else { "No (resume restarts from beginning)" },
        job.speed_limit.map(|n| format!("{}/sec", amount(n))).unwrap_or_else(|| "Unlimited".into()),
        date(job.created_at),
        date(job.last_try),
        date(job.finished_at),
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SegmentRow {
    pub id: u32,
    pub cells: [String; 5],
    /// Hundredths of a percent; None draws an indeterminate native progress bar.
    pub progress: Option<u32>,
    pub state: SegmentState,
}

pub fn format_segments(job: &JobSnapshot) -> Vec<SegmentRow> {
    let Some(details) = &job.transfer_details else {
        return vec![];
    };
    let mut rows: Vec<_> = details
        .segments
        .iter()
        .map(|segment| {
            let state = match job.state {
                JobState::Completed => SegmentState::Completed,
                JobState::Paused | JobState::Pausing if segment.state.is_active() => {
                    SegmentState::Paused
                }
                JobState::Cancelled | JobState::Cancelling if segment.state.is_active() => {
                    SegmentState::Cancelled
                }
                _ => segment.state,
            };
            let progress = if state == SegmentState::Completed {
                Some(10_000)
            } else {
                segment
                    .end
                    .and_then(|end| (end as u128).checked_sub(segment.start as u128))
                    .map(|length| {
                        ((segment.received as u128 * 10_000) / (length + 1)).min(10_000) as u32
                    })
            };
            let status = match state {
                SegmentState::Waiting => "Waiting",
                SegmentState::Connecting => "Connecting",
                SegmentState::Receiving => "Receiving",
                SegmentState::Paused => "Paused",
                SegmentState::Completed => "Complete",
                SegmentState::Failed => "Failed",
                SegmentState::Cancelled => "Stopped",
            };
            let speed = if job.state == JobState::Running && state == SegmentState::Receiving {
                segment.bytes_per_second
            } else {
                0
            };
            SegmentRow {
                id: segment.id,
                cells: [
                    (u64::from(segment.id) + 1).to_string(),
                    if job.total == Some(0) {
                        "Empty".into()
                    } else {
                        segment
                            .end
                            .map(|end| format!("{}–{end}", segment.start))
                            .unwrap_or_else(|| format!("{}–Unknown", segment.start))
                    },
                    progress
                        .map(|p| format!("{}.{:02} %", p / 100, p % 100))
                        .unwrap_or_else(|| "Unknown".into()),
                    status.into(),
                    format!("{}/sec", amount(speed)),
                ],
                progress,
                state,
            }
        })
        .collect();
    rows.sort_by_key(|row| row.id);
    rows
}

#[derive(Debug, PartialEq, Eq)]
pub enum SegmentChange {
    Remove(usize),
    Insert(usize),
    SetCell { row: usize, column: usize },
}

/// Computes incremental native-list operations without replacing existing rows.
#[derive(Default)]
pub struct SegmentTableRows {
    pub rows: Vec<SegmentRow>,
}
impl SegmentTableRows {
    pub fn sync(&mut self, next: Vec<SegmentRow>) -> Vec<SegmentChange> {
        let mut changes = Vec::new();
        for index in (0..self.rows.len()).rev() {
            if !next.iter().any(|row| row.id == self.rows[index].id) {
                self.rows.remove(index);
                changes.push(SegmentChange::Remove(index));
            }
        }
        for (index, row) in next.iter().enumerate() {
            let inserted = self.rows.get(index).is_none_or(|old| old.id != row.id);
            if inserted {
                self.rows.insert(index, row.clone());
                changes.push(SegmentChange::Insert(index));
            }
            for column in 0..5 {
                if inserted || self.rows[index].cells[column] != row.cells[column] {
                    changes.push(SegmentChange::SetCell { row: index, column });
                }
            }
            self.rows[index] = row.clone();
        }
        changes
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
