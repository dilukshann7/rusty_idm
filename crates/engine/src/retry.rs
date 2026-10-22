use serde::{Deserialize, Serialize};
use std::time::SystemTime;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RetryPolicy {
    /// Additional attempts after the initial attempt; zero disables recovery.
    pub max_retries: u8,
    pub delay_seconds: u16,
}
impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            delay_seconds: 5,
        }
    }
}
impl RetryPolicy {
    pub fn validate(&self) -> Result<(), String> {
        if self.max_retries > 100 {
            return Err("Automatic retries must be between 0 and 100.".into());
        }
        if !(1..=3600).contains(&self.delay_seconds) {
            return Err("Retry delay must be between 1 and 3600 seconds.".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RetryStatus {
    /// One-based number of the upcoming retry.
    pub attempt: u8,
    pub max_retries: u8,
    /// Display only; scheduling uses a monotonic deadline in the actor.
    /// None means an explicitly paused retry.
    pub next_retry_at: Option<SystemTime>,
}

#[cfg(test)]
#[path = "retry_tests.rs"]
mod tests;
