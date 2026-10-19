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
        }
    }
    fn matches(self, job: &JobSnapshot) -> bool {
        let extension = job
            .destination
            .extension()
            .unwrap_or_default()
            .to_string_lossy()
            .to_ascii_lowercase();
        if !matches!(self, Self::All | Self::Finished | Self::Unfinished) {
            if let Some(category) = &job.category {
                return category.eq_ignore_ascii_case(self.label());
            }
        }
        match self {
            Self::All => true,
            Self::Finished => job.state == JobState::Completed,
            Self::Unfinished => job.state != JobState::Completed,
            Self::Compressed => ["zip", "7z", "rar", "tar", "gz"].contains(&extension.as_str()),
            Self::Documents => ["pdf", "txt", "doc", "docx", "xls", "xlsx", "ppt", "pptx"]
                .contains(&extension.as_str()),
            Self::Music => ["mp3", "wav", "flac", "m4a"].contains(&extension.as_str()),
            Self::Programs => ["exe", "msi", "msix"].contains(&extension.as_str()),
            Self::Video => ["mp4", "mkv", "webm", "avi"].contains(&extension.as_str()),
        }
    }
}
#[derive(Default)]
pub struct ViewModel {
    pub jobs: BTreeMap<JobId, JobSnapshot>,
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
            _ => {}
        }
    }
    pub fn visible_jobs(&self, category: Category) -> Vec<JobId> {
        self.jobs
            .values()
            .filter(|s| category.matches(s))
            .map(|s| s.id)
            .collect()
    }
    pub fn actions(&self, selected: Option<JobId>) -> ActionAvailability {
        let job = selected.and_then(|id| self.jobs.get(&id));
        ActionAvailability {
            start: job.is_some_and(|s| {
                (s.state == JobState::Waiting && s.deferred) || s.state == JobState::Paused
            }),
            stop: job.is_some_and(|s| s.state.is_active() && s.state != JobState::Cancelling),
            stop_all: self
                .jobs
                .values()
                .any(|s| s.state.is_active() && s.state != JobState::Cancelling),
            remove: job.is_some_and(|s| s.state != JobState::Cancelling),
            retry: job.is_some_and(|s| {
                s.state.can_retry()
                    && s.error
                        .as_ref()
                        .is_none_or(|e| e.retained_partial.is_none())
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
                let path = error.retained_partial.as_ref()?;
                Some(format!(
                    "{}\r\n{}\r\nPartial: {}",
                    job.filename,
                    error.message,
                    path.display()
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
    let state = match job.state {
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
    };
    let description = job
        .error
        .as_ref()
        .map(|e| match &e.retained_partial {
            Some(path) => format!("{} Partial: {}", e.message, path.display()),
            None => e.message.clone(),
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
