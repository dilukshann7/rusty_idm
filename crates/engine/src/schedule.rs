//! Civil-time schedule rules, independent of the GUI and transfer executor.
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Date {
    pub year: u16,
    pub month: u8,
    pub day: u8,
}
impl Date {
    pub fn valid(self) -> bool {
        (1601..=9999).contains(&self.year)
            && (1..=12).contains(&self.month)
            && self.day > 0
            && self.day <= self.month_days()
    }
    fn month_days(self) -> u8 {
        match self.month {
            2 => {
                if self.year % 4 == 0 && (self.year % 100 != 0 || self.year % 400 == 0) {
                    29
                } else {
                    28
                }
            }
            4 | 6 | 9 | 11 => 30,
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            _ => 0,
        }
    }
    fn ordinal(self) -> u32 {
        let year = self.year as u32 - 1;
        let mut days = year * 365 + year / 4 - year / 100 + year / 400;
        for month in 1..self.month {
            days += Self { month, ..self }.month_days() as u32;
        }
        days + self.day as u32
    }
    /// Sunday=0, matching SYSTEMTIME and the weekday checkboxes.
    pub fn weekday(self) -> u8 {
        (self.ordinal() % 7) as u8
    }
    pub fn next(self) -> Option<Self> {
        if !self.valid() {
            return None;
        }
        if self.day < self.month_days() {
            Some(Self {
                day: self.day + 1,
                ..self
            })
        } else if self.month < 12 {
            Some(Self {
                month: self.month + 1,
                day: 1,
                ..self
            })
        } else if self.year < 9999 {
            Some(Self {
                year: self.year + 1,
                month: 1,
                day: 1,
            })
        } else {
            None
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LocalStamp {
    pub date: Date,
    pub minute: u16,
}
impl LocalStamp {
    pub fn valid(self) -> bool {
        self.date.valid() && self.minute < 1440
    }
    pub fn label(self) -> String {
        format!(
            "{:04}-{:02}-{:02} {:02}:{:02}",
            self.date.year,
            self.date.month,
            self.date.day,
            self.minute / 60,
            self.minute % 60
        )
    }
    pub fn now() -> Option<Self> {
        local_now()
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum StartRule {
    Once(LocalStamp),
    Weekly { minute: u16, days: u8 },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct QueueSchedule {
    pub start_on_startup: bool,
    pub enabled: bool,
    pub start: StartRule,
    pub stop_minute: Option<u16>,
}
impl Default for QueueSchedule {
    fn default() -> Self {
        Self {
            start_on_startup: false,
            enabled: false,
            start: StartRule::Weekly {
                minute: 9 * 60,
                days: 0x7f,
            },
            stop_minute: None,
        }
    }
}
impl QueueSchedule {
    pub fn validate(&self) -> Result<(), String> {
        let minute = match self.start {
            StartRule::Once(at) => {
                if !at.valid() {
                    return Err("Choose a valid start date and time.".into());
                }
                at.minute
            }
            StartRule::Weekly { minute, days } => {
                if minute >= 1440 || days == 0 || days & !0x7f != 0 {
                    return Err("Choose a valid start time and at least one weekday.".into());
                }
                minute
            }
        };
        if self.stop_minute.is_some_and(|m| m >= 1440) {
            return Err("Choose a valid stop time.".into());
        }
        if self.enabled && self.stop_minute == Some(minute) {
            return Err("The start and stop times must be different.".into());
        }
        Ok(())
    }
    pub fn validate_for_apply(&self, now: LocalStamp) -> Result<(), String> {
        self.validate()?;
        if self.enabled && matches!(self.start,StartRule::Once(at)if at<now) {
            return Err("Choose a one-time start that has not already passed.".into());
        }
        Ok(())
    }
    pub fn due(&self, now: LocalStamp, last: Option<LocalStamp>) -> bool {
        self.enabled
            && now.valid()
            && last.is_none_or(|last| now > last)
            && match self.start {
                StartRule::Once(at) => at == now,
                StartRule::Weekly { minute, days } => {
                    minute == now.minute && days & (1 << now.date.weekday()) != 0
                }
            }
    }
    pub fn next_start(&self, now: LocalStamp, last: Option<LocalStamp>) -> Option<LocalStamp> {
        if !self.enabled {
            return None;
        }
        match self.start {
            StartRule::Once(at) => (at >= now && last.is_none_or(|last| at > last)).then_some(at),
            StartRule::Weekly { minute, days } => {
                let mut date = last
                    .filter(|last| *last > now)
                    .map_or(now.date, |last| last.date);
                for _ in 0..8 {
                    let at = LocalStamp { date, minute };
                    if at >= now
                        && last.is_none_or(|last| at > last)
                        && days & (1 << date.weekday()) != 0
                    {
                        return Some(at);
                    }
                    date = date.next()?;
                }
                None
            }
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueueSettings {
    pub parallelism: u8,
    pub schedule: QueueSchedule,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ScheduleStatus {
    pub next_start: Option<LocalStamp>,
    pub stop_at: Option<LocalStamp>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct ScheduleRecord {
    pub config: QueueSchedule,
    pub generation: u64,
    pub last_fired: Option<LocalStamp>,
}
/// Absolute stop instant, with a civil label only for display.
#[derive(Clone, Copy, Debug)]
pub(crate) struct StopDeadline {
    pub local: LocalStamp,
    pub utc_ticks: u64,
}
impl StopDeadline {
    pub fn next(minute: u16) -> Option<Self> {
        if minute >= 1440 {
            return None;
        }
        let now = ClockSample::now()?;
        let mut date = now.local.date;
        // A clock gap can remove today's occurrence. Search the next valid day.
        for _ in 0..3 {
            let local = LocalStamp { date, minute };
            if let Some(utc_ticks) = local_candidates(local)?
                .into_iter()
                .filter(|ticks| *ticks > now.utc_ticks)
                .min()
            {
                return Some(Self { local, utc_ticks });
            }
            date = date.next()?;
        }
        None
    }
}
pub(crate) struct ClockSample {
    pub local: LocalStamp,
    pub utc_ticks: u64,
}
impl ClockSample {
    pub fn now() -> Option<Self> {
        let elapsed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?;
        let utc_ticks = elapsed
            .as_secs()
            .checked_add(11_644_473_600)?
            .checked_mul(10_000_000)?
            .checked_add(elapsed.subsec_nanos() as u64 / 100)?;
        Some(Self {
            local: local_from_ticks(utc_ticks)?,
            utc_ticks,
        })
    }
}
fn local_now() -> Option<LocalStamp> {
    Some(ClockSample::now()?.local)
}
#[cfg(windows)]
fn local_from_ticks(ticks: u64) -> Option<LocalStamp> {
    use windows::Win32::{
        Foundation::{FILETIME, SYSTEMTIME},
        System::Time::{FileTimeToSystemTime, SystemTimeToTzSpecificLocalTimeEx},
    };
    let file = FILETIME {
        dwLowDateTime: ticks as u32,
        dwHighDateTime: (ticks >> 32) as u32,
    };
    let mut utc = SYSTEMTIME::default();
    let mut local = SYSTEMTIME::default();
    unsafe {
        FileTimeToSystemTime(&file, &mut utc).ok()?;
        SystemTimeToTzSpecificLocalTimeEx(None, &utc, &mut local).ok()?;
    }
    let stamp = LocalStamp {
        date: Date {
            year: local.wYear,
            month: local.wMonth as u8,
            day: local.wDay as u8,
        },
        minute: local.wHour * 60 + local.wMinute,
    };
    stamp.valid().then_some(stamp)
}
#[cfg(windows)]
fn local_candidates(stamp: LocalStamp) -> Option<Vec<u64>> {
    use windows::Win32::{
        Foundation::{FILETIME, SYSTEMTIME},
        System::Time::{
            FileTimeToSystemTime, GetDynamicTimeZoneInformation, GetTimeZoneInformationForYear,
            SystemTimeToFileTime, SystemTimeToTzSpecificLocalTimeEx, DYNAMIC_TIME_ZONE_INFORMATION,
            TIME_ZONE_INFORMATION,
        },
    };
    if !stamp.valid() {
        return None;
    }
    let mut zone = DYNAMIC_TIME_ZONE_INFORMATION::default();
    let mut rules = TIME_ZONE_INFORMATION::default();
    let local = SYSTEMTIME {
        wYear: stamp.date.year,
        wMonth: stamp.date.month as u16,
        wDay: stamp.date.day as u16,
        wHour: stamp.minute / 60,
        wMinute: stamp.minute % 60,
        ..Default::default()
    };
    let mut encoded = FILETIME::default();
    unsafe {
        if GetDynamicTimeZoneInformation(&mut zone) == u32::MAX {
            return None;
        }
        GetTimeZoneInformationForYear(stamp.date.year, Some(&zone), &mut rules).ok()?;
        SystemTimeToFileTime(&local, &mut encoded).ok()?;
    }
    let wall_ticks = ((encoded.dwHighDateTime as u64) << 32) | encoded.dwLowDateTime as u64;
    let mut candidates = vec![];
    // Try both offsets and round-trip each through Windows' historical rules.
    // This retains both repeated-hour occurrences and rejects spring gaps.
    // https://learn.microsoft.com/en-us/windows/win32/sysinfo/local-time
    for extra in [rules.StandardBias, rules.DaylightBias] {
        let ticks = wall_ticks as i128 + (rules.Bias as i128 + extra as i128) * 600_000_000;
        let Ok(ticks) = u64::try_from(ticks) else {
            continue;
        };
        let file = FILETIME {
            dwLowDateTime: ticks as u32,
            dwHighDateTime: (ticks >> 32) as u32,
        };
        let mut utc = SYSTEMTIME::default();
        let mut roundtrip = SYSTEMTIME::default();
        unsafe {
            FileTimeToSystemTime(&file, &mut utc).ok()?;
            SystemTimeToTzSpecificLocalTimeEx(Some(&zone), &utc, &mut roundtrip).ok()?;
        }
        if roundtrip.wYear == local.wYear
            && roundtrip.wMonth == local.wMonth
            && roundtrip.wDay == local.wDay
            && roundtrip.wHour == local.wHour
            && roundtrip.wMinute == local.wMinute
            && roundtrip.wSecond == 0
        {
            candidates.push(ticks);
        }
    }
    Some(candidates)
}
#[cfg(not(windows))]
fn local_from_ticks(ticks: u64) -> Option<LocalStamp> {
    // Portable engine consumers use UTC; the desktop target uses Windows local time.
    let seconds = ticks.checked_div(10_000_000)?.checked_sub(11_644_473_600)?;
    let mut day = seconds / 86400;
    let mut date = Date {
        year: 1970,
        month: 1,
        day: 1,
    };
    while day > 0 {
        date = date.next()?;
        day -= 1;
    }
    Some(LocalStamp {
        date,
        minute: ((seconds % 86400) / 60) as u16,
    })
}
#[cfg(not(windows))]
fn local_candidates(stamp: LocalStamp) -> Option<Vec<u64>> {
    if !stamp.valid() {
        return None;
    }
    let epoch = Date {
        year: 1601,
        month: 1,
        day: 1,
    }
    .ordinal();
    Some(vec![
        (stamp.date.ordinal() - epoch) as u64 * 864_000_000_000 + stamp.minute as u64 * 600_000_000,
    ])
}
