use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

/// Average transfer progress over recent ticks so a burst of network reads
/// does not appear as a spike followed by a row of zero-speed updates.
pub(super) struct SpeedMeter {
    samples: VecDeque<(Instant, u64)>,
}

impl SpeedMeter {
    const WINDOW: Duration = Duration::from_secs(2);
    const STARTUP_SPAN: Duration = Duration::from_secs(1);

    pub(super) fn new(now: Instant, received: u64) -> Self {
        Self {
            samples: VecDeque::from([(now, received)]),
        }
    }

    pub(super) fn reset(&mut self, now: Instant, received: u64) {
        self.samples.clear();
        self.samples.push_back((now, received));
    }

    pub(super) fn sample(&mut self, now: Instant, received: u64) -> u64 {
        if self
            .samples
            .back()
            .is_some_and(|(_, previous)| received < *previous)
        {
            self.reset(now, received);
            return 0;
        }
        self.samples.push_back((now, received));
        while self.samples.len() > 1 && now.duration_since(self.samples[1].0) >= Self::WINDOW {
            self.samples.pop_front();
        }
        let (start, start_bytes) = self.samples[0];
        let seconds = now
            .duration_since(start)
            .max(Self::STARTUP_SPAN)
            .as_secs_f64();
        ((received - start_bytes) as f64 / seconds) as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bursty_progress_stays_nonzero_between_reads_then_expires() {
        let start = Instant::now();
        let mut meter = SpeedMeter::new(start, 0);
        let mib = 1_048_576;
        assert_eq!(meter.sample(start + Duration::from_secs(1), mib), mib);
        assert!(meter.sample(start + Duration::from_millis(1500), mib) > 0);
        assert!(meter.sample(start + Duration::from_secs(2), mib * 2) > 0);
        assert!(meter.sample(start + Duration::from_millis(2500), mib * 2) > 0);
        assert_eq!(meter.sample(start + Duration::from_secs(5), mib * 2), 0);
    }

    #[test]
    fn reset_drops_speed_from_the_previous_run() {
        let start = Instant::now();
        let mut meter = SpeedMeter::new(start, 0);
        meter.sample(start + Duration::from_secs(1), 1_048_576);
        meter.reset(start + Duration::from_secs(2), 1_048_576);
        assert_eq!(
            meter.sample(start + Duration::from_millis(2100), 1_048_576),
            0
        );
    }
}
