use super::speed::SpeedMeter;
use crate::{ConnectionProgress, JobState, SegmentSnapshot, SegmentState, TransferDetails};
use std::{collections::BTreeMap, time::Instant};

#[derive(Default)]
pub(super) struct TransferMeter {
    latest: Option<ConnectionProgress>,
    meters: BTreeMap<u32, (u64, SpeedMeter)>,
}
impl TransferMeter {
    /// Record worker shutdown before automatic recovery changes the job to Waiting.
    pub(super) fn finish(&mut self, now: Instant, state: JobState) {
        if let Some(view) = self.snapshot(now, state) {
            for (raw, row) in self
                .latest
                .as_mut()
                .expect("telemetry")
                .segments
                .iter_mut()
                .zip(view.segments)
            {
                raw.state = row.state;
            }
        }
    }
    pub(super) fn update(&mut self, progress: ConnectionProgress, now: Instant) {
        self.meters
            .retain(|id, _| progress.segments.iter().any(|s| s.id == *id));
        for row in &progress.segments {
            let meter = self
                .meters
                .entry(row.id)
                .or_insert_with(|| (row.generation, SpeedMeter::new(now, 0)));
            if meter.0 != row.generation {
                *meter = (row.generation, SpeedMeter::new(now, 0));
            }
        }
        self.latest = Some(progress);
    }

    pub(super) fn snapshot(&mut self, now: Instant, state: JobState) -> Option<TransferDetails> {
        let progress = self.latest.as_ref()?;
        let identified_failure = progress
            .segments
            .iter()
            .any(|s| s.state == SegmentState::Failed);
        let segments: Vec<_> = progress
            .segments
            .iter()
            .map(|row| {
                let row_state = match (state, row.state) {
                    (JobState::Completed, _) => SegmentState::Completed,
                    (
                        _,
                        SegmentState::Completed | SegmentState::Failed | SegmentState::Cancelled,
                    ) => row.state,
                    (JobState::Paused | JobState::Pausing, _) => SegmentState::Paused,
                    (JobState::Cancelled | JobState::Cancelling, _) => SegmentState::Cancelled,
                    (JobState::Failed, _) if !identified_failure && row.state.is_active() => {
                        SegmentState::Failed
                    }
                    (JobState::Failed | JobState::Waiting, _) => SegmentState::Cancelled,
                    _ => row.state,
                };
                let meter = &mut self.meters.get_mut(&row.id).expect("worker meter").1;
                let speed = if state == JobState::Running && row_state == SegmentState::Receiving {
                    meter.sample(now, row.transferred)
                } else {
                    meter.reset(now, row.transferred);
                    0
                };
                SegmentSnapshot {
                    id: row.id,
                    start: row.start,
                    end: row.end,
                    received: row.received,
                    state: row_state,
                    bytes_per_second: speed,
                }
            })
            .collect();
        Some(TransferDetails {
            connection_limit: progress.connection_limit,
            active_connections: segments.iter().filter(|s| s.state.is_active()).count(),
            segments,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ConnectionProgress, JobState, SegmentProgress, SegmentState};
    use std::time::{Duration, Instant};

    fn row(id: u32, received: u64, transferred: u64) -> SegmentProgress {
        SegmentProgress {
            id,
            start: u64::from(id) * 10_000,
            end: Some(u64::from(id) * 10_000 + 9_999),
            received,
            transferred,
            generation: 1,
            state: SegmentState::Receiving,
        }
    }
    fn progress(segments: Vec<SegmentProgress>) -> ConnectionProgress {
        ConnectionProgress {
            connection_limit: 4,
            segments,
        }
    }

    #[test]
    fn saved_bytes_are_not_speed_and_idle_connections_decay() {
        let now = Instant::now();
        let mut meter = TransferMeter::default();
        meter.update(progress(vec![row(0, 8000, 0), row(1, 0, 0)]), now);
        assert_eq!(
            meter
                .snapshot(now, JobState::Running)
                .unwrap()
                .active_connections,
            2
        );
        meter.update(
            progress(vec![row(0, 9000, 1000), row(1, 2000, 2000)]),
            now + Duration::from_secs(1),
        );
        let view = meter
            .snapshot(now + Duration::from_secs(1), JobState::Running)
            .unwrap();
        assert_eq!(view.segments[0].bytes_per_second, 1000);
        assert_eq!(view.segments[1].bytes_per_second, 2000);
        let idle = meter
            .snapshot(now + Duration::from_secs(5), JobState::Running)
            .unwrap();
        assert!(idle.segments.iter().all(|s| s.bytes_per_second == 0));
    }

    #[test]
    fn pause_terminal_and_failed_worker_preserve_progress_without_live_connections() {
        let now = Instant::now();
        let mut meter = TransferMeter::default();
        meter.update(progress(vec![row(0, 500, 500), row(1, 100, 100)]), now);
        let paused = meter.snapshot(now, JobState::Paused).unwrap();
        assert_eq!(paused.active_connections, 0);
        assert_eq!(paused.segments[0].received, 500);
        assert!(paused
            .segments
            .iter()
            .all(|s| s.state == SegmentState::Paused && s.bytes_per_second == 0));
        let mut failed = row(1, 100, 100);
        failed.state = SegmentState::Failed;
        meter.update(progress(vec![row(0, 500, 500), failed]), now);
        let view = meter.snapshot(now, JobState::Failed).unwrap();
        assert_eq!(view.active_connections, 0);
        assert_eq!(view.segments[1].state, SegmentState::Failed);
        assert_eq!(view.segments[0].state, SegmentState::Cancelled);
        assert!(view.segments.iter().all(|s| s.bytes_per_second == 0));
    }

    #[test]
    fn resume_resets_speed_and_completed_ranges_do_not_measure_assembly() {
        let now = Instant::now();
        let mut meter = TransferMeter::default();
        meter.update(progress(vec![row(0, 0, 0)]), now);
        meter.update(
            progress(vec![row(0, 4000, 4000)]),
            now + Duration::from_secs(1),
        );
        assert!(
            meter
                .snapshot(now + Duration::from_secs(1), JobState::Running)
                .unwrap()
                .segments[0]
                .bytes_per_second
                > 0
        );
        let mut resumed = row(0, 4000, 0);
        resumed.generation = 2;
        meter.update(
            progress(vec![resumed.clone()]),
            now + Duration::from_secs(2),
        );
        assert_eq!(
            meter
                .snapshot(now + Duration::from_secs(2), JobState::Running)
                .unwrap()
                .segments[0]
                .bytes_per_second,
            0
        );
        resumed.received = 10_000;
        resumed.transferred = 6000;
        resumed.state = SegmentState::Completed;
        meter.update(progress(vec![resumed]), now + Duration::from_secs(3));
        let assembled = meter
            .snapshot(now + Duration::from_secs(3), JobState::Running)
            .unwrap();
        assert_eq!(assembled.active_connections, 0);
        assert_eq!(assembled.segments[0].bytes_per_second, 0);
        assert!(TransferMeter::default()
            .snapshot(now, JobState::Completed)
            .is_none());
    }
}
