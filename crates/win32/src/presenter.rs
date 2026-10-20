use rusty_engine::{Event, JobId, JobSnapshot, JobState};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Category {
    #[default]
    All,
    Compressed,
    Documents,
    Music,
    Programs,
    Video,
    Unfinished,
    Finished,
    Filtered(u8, u8),
    QueueRoot,
    Queue(rusty_engine::QueueId),
    Custom(u64),
}
impl Category {
    pub const ALL: [Self; 8] = [
        Self::All,
        Self::Compressed,
        Self::Documents,
        Self::Music,
        Self::Programs,
        Self::Video,
        Self::Unfinished,
        Self::Finished,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::All => "All Downloads",
            Self::Compressed => "Compressed",
            Self::Documents => "Documents",
            Self::Music => "Music",
            Self::Programs => "Programs",
            Self::Video => "Video",
            Self::Unfinished => "Unfinished",
            Self::Finished => "Finished",
            Self::Filtered(_, kind) => rusty_engine::categories::BUILTIN[kind as usize],
            Self::QueueRoot => "Queues",
            Self::Queue(_) => "Queue",
            Self::Custom(_) => "Custom category",
        }
    }
    fn matches(self, job: &JobSnapshot, rules: &[rusty_engine::categories::CategoryRule]) -> bool {
        let category = job
            .category
            .as_deref()
            .filter(|c| !c.eq_ignore_ascii_case("Automatic"))
            .or_else(|| {
                if rules.is_empty() {
                    rusty_engine::categories::infer(&job.destination)
                } else {
                    rusty_engine::categories::classify(&job.destination, rules)
                        .map(|r| r.name.as_str())
                }
            });
        match self {
            Self::All => true,
            Self::Finished => job.state == JobState::Completed,
            Self::Unfinished => job.state != JobState::Completed,
            Self::QueueRoot => job.queue_id.is_some(),
            Self::Queue(id) => job.queue_id == Some(id),
            Self::Custom(hash) => category.is_some_and(|c| category_key(c) == hash),
            Self::Filtered(status, kind) => {
                (status == 0 || (job.state == JobState::Completed) == (status == 2))
                    && category.is_some_and(|c| {
                        c.eq_ignore_ascii_case(rusty_engine::categories::BUILTIN[kind as usize])
                    })
            }
            other => category.is_some_and(|c| c.eq_ignore_ascii_case(other.label())),
        }
    }
}
#[derive(Default)]
pub struct ViewModel {
    pub jobs: BTreeMap<JobId, JobSnapshot>,
    pub categories: Vec<rusty_engine::categories::CategoryRule>,
    pub queues: BTreeMap<rusty_engine::QueueId, rusty_engine::QueueSnapshot>,
}
#[derive(Default)]
pub struct ActionAvailability {
    pub start: bool,
    pub stop: bool,
    pub stop_all: bool,
    pub remove: bool,
    pub retry: bool,
    pub open_file: bool,
    pub open_folder: bool,
    pub details: bool,
}
impl ViewModel {
    pub fn apply(&mut self, event: Event) {
        match event {
            Event::JobChanged(s) => {
                self.jobs.insert(s.id, s);
            }
            Event::JobRemoved(id) => {
                self.jobs.remove(&id);
            }
            Event::QueuesChanged(queues) => {
                self.queues = queues.into_iter().map(|q| (q.id, q)).collect()
            }
            _ => {}
        }
    }
    pub fn visible_jobs(&self, category: Category) -> Vec<JobId> {
        let mut jobs: Vec<_> = self
            .jobs
            .values()
            .filter(|s| category.matches(s, &self.categories))
            .map(|s| s.id)
            .collect();
        if let Category::Queue(id) = category {
            if let Some(queue) = self.queues.get(&id) {
                jobs.sort_by_key(|id| {
                    self.jobs[id]
                        .entry_id
                        .and_then(|entry| queue.entries.iter().position(|e| *e == entry))
                        .unwrap_or(usize::MAX)
                });
            }
        }
        jobs
    }
    pub fn actions(&self, selected: Option<JobId>) -> ActionAvailability {
        let job = selected.and_then(|id| self.jobs.get(&id));
        ActionAvailability {
            start: job.is_some_and(|s| {
                s.readiness == rusty_engine::EntryReadiness::ConfirmationRequired
                    || (s.state == JobState::Waiting && s.deferred)
                    || s.state == JobState::Paused
                    || (s.state == JobState::Failed
                        && s.entry_id.is_some()
                        && s.readiness == rusty_engine::EntryReadiness::Ready)
            }),
            stop: job.is_some_and(|s| s.state.is_active() && s.state != JobState::Cancelling),
            stop_all: self
                .jobs
                .values()
                .any(|s| s.state.is_active() && s.state != JobState::Cancelling),
            remove: job.is_some_and(|s| s.state != JobState::Cancelling),
            retry: job.is_some_and(|s| {
                s.state.can_retry()
                    && s.error.as_ref().is_none_or(|e| {
                        e.retained_partial.is_none() && e.retained_original.is_none()
                    })
            }),
            open_file: job.is_some_and(|s| s.state == JobState::Completed),
            open_folder: job.is_some_and(|s| s.state.is_terminal()),
            details: job.is_some(),
        }
    }
    pub fn active(&self) -> bool {
        self.jobs.values().any(|s| s.state.is_active())
    }
    pub fn recovery_notice(&self) -> Option<String> {
        let lines: Vec<_> = self
            .jobs
            .values()
            .filter_map(|job| {
                let error = job.error.as_ref()?;
                if error.retained_partial.is_none() && error.retained_original.is_none() {
                    return None;
                }
                Some(format!(
                    "{}\r\n{}{}{}",
                    job.filename,
                    error.message,
                    error
                        .retained_partial
                        .as_ref()
                        .map(|p| format!("\r\nPartial: {}", p.display()))
                        .unwrap_or_default(),
                    error
                        .retained_original
                        .as_ref()
                        .map(|p| format!("\r\nOriginal backup: {}", p.display()))
                        .unwrap_or_default()
                ))
            })
            .collect();
        if lines.is_empty() {
            None
        } else {
            Some(lines.join("\r\n\r\n"))
        }
    }
    pub fn aggregate_speed(&self) -> u64 {
        self.jobs
            .values()
            .filter(|s| s.state == JobState::Running)
            .fold(0u64, |sum, s| sum.saturating_add(s.bytes_per_second))
    }
}
pub struct RowText {
    pub columns: [String; 7],
}
pub fn format_snapshot(job: &JobSnapshot) -> RowText {
    let state = match job.readiness {
        rusty_engine::EntryReadiness::ConfirmationRequired => {
            "Confirm this download before starting".into()
        }
        rusty_engine::EntryReadiness::CredentialsRequired => "Credentials required".into(),
        rusty_engine::EntryReadiness::RestartRequired => "Restart required".into(),
        rusty_engine::EntryReadiness::RecoveryRequired => "Recovery required".into(),
        rusty_engine::EntryReadiness::Ready => match job.state {
            JobState::Waiting if job.deferred => "Download later".into(),
            JobState::Running => job
                .total
                .filter(|n| *n > 0)
                .map(|n| {
                    format!(
                        "Downloading — {}%",
                        ((job.received as u128 * 100) / n as u128).min(100)
                    )
                })
                .unwrap_or("Downloading".into()),
            state => format!("{state:?}"),
        },
    };
    let description = job
        .error
        .as_ref()
        .map(|e| {
            format!(
                "{}{}{}",
                e.message,
                e.retained_partial
                    .as_ref()
                    .map(|p| format!(" Partial: {}", p.display()))
                    .unwrap_or_default(),
                e.retained_original
                    .as_ref()
                    .map(|p| format!(" Original backup: {}", p.display()))
                    .unwrap_or_default()
            )
        })
        .unwrap_or_else(|| {
            if job.description.is_empty() {
                job.source_host.clone()
            } else {
                job.description.clone()
            }
        });
    RowText {
        columns: [
            job.filename.clone(),
            job.total
                .map(format_bytes)
                .unwrap_or_else(|| format!("{} received", format_bytes(job.received))),
            state,
            if job.total.is_some() && job.state == JobState::Running {
                job.eta_seconds.map(format_duration).unwrap_or_default()
            } else {
                String::new()
            },
            if job.state == JobState::Running {
                format!("{}/s", format_bytes(job.bytes_per_second))
            } else {
                String::new()
            },
            job.last_try.map(local_time).unwrap_or_default(),
            description,
        ],
    }
}
pub fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut number = bytes as f64;
    let mut unit = "B";
    for next in ["KiB", "MiB", "GiB", "TiB", "PiB", "EiB"] {
        if number < 1024.0 {
            break;
        }
        number /= 1024.0;
        unit = next;
    }
    format!("{number:.1} {unit}")
}
fn format_duration(seconds: u64) -> String {
    format!(
        "{}:{:02}:{:02}",
        seconds / 3600,
        seconds / 60 % 60,
        seconds % 60
    )
}
#[cfg(windows)]
fn local_time(time: std::time::SystemTime) -> String {
    use windows::Win32::{
        Foundation::{FILETIME, SYSTEMTIME},
        System::Time::{FileTimeToSystemTime, SystemTimeToTzSpecificLocalTime},
    };
    let Ok(duration) = time.duration_since(std::time::UNIX_EPOCH) else {
        return String::new();
    };
    let ticks = duration
        .as_secs()
        .saturating_add(11_644_473_600)
        .saturating_mul(10_000_000)
        + duration.subsec_nanos() as u64 / 100;
    let filetime = FILETIME {
        dwLowDateTime: ticks as u32,
        dwHighDateTime: (ticks >> 32) as u32,
    };
    let mut utc = SYSTEMTIME::default();
    let mut local = SYSTEMTIME::default();
    unsafe {
        if FileTimeToSystemTime(&filetime, &mut utc).is_err()
            || SystemTimeToTzSpecificLocalTime(None, &utc, &mut local).is_err()
        {
            return String::new();
        }
    }
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        local.wYear, local.wMonth, local.wDay, local.wHour, local.wMinute, local.wSecond
    )
}
#[cfg(not(windows))]
fn local_time(_: std::time::SystemTime) -> String {
    String::new()
}

#[cfg(test)]
#[path = "presenter_tests.rs"]
mod tests;

pub fn category_key(name: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    name.to_ascii_lowercase().hash(&mut h);
    h.finish()
}
