use crate::{DownloadRequest, JobId, JobSnapshot};
use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};

#[derive(Clone, Debug)]
pub enum Event {
    JobChanged(JobSnapshot),
    JobRemoved(JobId),
    QueuesChanged(Vec<crate::QueueSnapshot>),
    CommandRejected { message: String },
    ShutdownComplete,
}
#[derive(Default)]
struct Pending {
    latest: Option<JobSnapshot>,
    terminal: Option<JobSnapshot>,
    removed: bool,
}
#[derive(Default)]
struct Queue {
    jobs: BTreeMap<JobId, Pending>,
    error: Option<String>,
    shutdown: bool,
    queues: Option<Vec<crate::QueueSnapshot>>,
    // Private requests never leave the engine; snapshots expose no credentials.
    known: BTreeMap<JobId, (DownloadRequest, Option<JobSnapshot>)>,
}
pub(crate) struct EventBus {
    queue: Mutex<Queue>,
    pending: AtomicBool,
    wake: Arc<dyn Fn() + Send + Sync>,
}
impl EventBus {
    pub fn register_request(&self, id: JobId, request: &DownloadRequest) {
        self.queue
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .known
            .insert(id, (request.clone(), None));
    }
    pub fn find_duplicate(&self, request: &DownloadRequest) -> Option<JobSnapshot> {
        self.queue
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .known
            .values()
            .rev()
            .find_map(|(existing, snapshot)| {
                existing
                    .same_source(request)
                    .then(|| snapshot.clone())
                    .flatten()
            })
    }
    pub fn destinations(&self) -> Vec<std::path::PathBuf> {
        self.queue
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .known
            .values()
            .map(|(request, _)| request.destination.clone())
            .collect()
    }
    pub fn new(wake: Arc<dyn Fn() + Send + Sync>) -> Self {
        Self {
            queue: Mutex::new(Queue::default()),
            pending: AtomicBool::new(false),
            wake,
        }
    }
    pub fn publish(&self, event: Event) {
        {
            let mut queue = self.queue.lock().unwrap_or_else(|p| p.into_inner());
            match event {
                Event::JobChanged(snapshot) => {
                    if let Some((_, latest)) = queue.known.get_mut(&snapshot.id) {
                        *latest = Some(snapshot.clone());
                    }
                    let slot = queue.jobs.entry(snapshot.id).or_default();
                    if snapshot.state.is_terminal() {
                        slot.latest = None;
                        slot.terminal = Some(snapshot);
                    } else if slot.terminal.is_none() {
                        slot.latest = Some(snapshot);
                    }
                }
                Event::JobRemoved(id) => {
                    queue.known.remove(&id);
                    queue.jobs.entry(id).or_default().removed = true;
                }
                Event::CommandRejected { message } => queue.error = Some(message),
                Event::ShutdownComplete => queue.shutdown = true,
                Event::QueuesChanged(queues) => queue.queues = Some(queues),
            }
        }
        if !self.pending.swap(true, Ordering::AcqRel) {
            (self.wake)();
        }
    }
    pub fn terminal_pending(&self, id: JobId) -> bool {
        self.queue
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .jobs
            .get(&id)
            .is_some_and(|s| s.terminal.is_some())
    }
    pub fn removed_pending(&self) -> usize {
        self.queue
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .jobs
            .values()
            .filter(|s| s.removed)
            .count()
    }
    pub fn drain(&self) -> Vec<Event> {
        let mut queue = self.queue.lock().unwrap_or_else(|p| p.into_inner());
        let mut result = Vec::new();
        for (id, slot) in std::mem::take(&mut queue.jobs) {
            if let Some(snapshot) = slot.latest {
                result.push(Event::JobChanged(snapshot));
            }
            if let Some(snapshot) = slot.terminal {
                result.push(Event::JobChanged(snapshot));
            }
            if slot.removed {
                result.push(Event::JobRemoved(id));
            }
        }
        if let Some(queues) = queue.queues.take() {
            result.push(Event::QueuesChanged(queues));
        }
        if let Some(message) = queue.error.take() {
            result.push(Event::CommandRejected { message });
        }
        if queue.shutdown {
            result.push(Event::ShutdownComplete);
            queue.shutdown = false;
        }
        // Clearing under the same lock as the drain makes a concurrent publisher
        // either part of this batch or responsible for a fresh wake notification.
        self.pending.store(false, Ordering::Release);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn snapshot(received: u64, state: crate::JobState) -> JobSnapshot {
        JobSnapshot {
            entry_id: None,
            restored: false,
            recovery_paths: vec![],
            queue_id: None,
            readiness: crate::EntryReadiness::Ready,
            deferred: false,
            category: None,
            description: String::new(),
            id: JobId(1),
            filename: "file.bin".into(),
            source_host: "example.com".into(),
            display_url: "https://example.com/file.bin".into(),
            destination: "file.bin".into(),
            state,
            received,
            total: None,
            resume_capable: false,
            speed_limit: None,
            bytes_per_second: 0,
            eta_seconds: None,
            last_try: None,
            error: None,
        }
    }
    #[test]
    fn burst_progress_is_bounded_and_terminal_precedes_removal_and_shutdown() {
        let bus = EventBus::new(Arc::new(|| {}));
        for received in 0..100_000 {
            bus.publish(Event::JobChanged(snapshot(
                received,
                crate::JobState::Running,
            )));
        }
        bus.publish(Event::JobChanged(snapshot(
            100_000,
            crate::JobState::Cancelled,
        )));
        bus.publish(Event::JobRemoved(JobId(1)));
        bus.publish(Event::ShutdownComplete);
        let events = bus.drain();
        assert_eq!(events.len(), 3);
        assert!(
            matches!(&events[0], Event::JobChanged(s) if s.received == 100_000 && s.state == crate::JobState::Cancelled)
        );
        assert!(matches!(events[1], Event::JobRemoved(JobId(1))));
        assert!(matches!(events[2], Event::ShutdownComplete));
        assert!(bus.drain().is_empty());
    }
    #[test]
    fn concurrent_errors_and_drain_do_not_lose_final_shutdown() {
        let wakes = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let count = wakes.clone();
        let bus = Arc::new(EventBus::new(Arc::new(move || {
            count.fetch_add(1, Ordering::SeqCst);
        })));
        let sender = bus.clone();
        let worker = std::thread::spawn(move || {
            for _ in 0..10000 {
                sender.publish(Event::CommandRejected {
                    message: "bounded".into(),
                });
            }
            sender.publish(Event::ShutdownComplete);
        });
        let mut done = false;
        while !worker.is_finished() {
            done |= bus
                .drain()
                .iter()
                .any(|e| matches!(e, Event::ShutdownComplete));
        }
        worker.join().unwrap();
        done |= bus
            .drain()
            .iter()
            .any(|e| matches!(e, Event::ShutdownComplete));
        assert!(done);
        assert!(wakes.load(Ordering::SeqCst) > 0);
    }
}
