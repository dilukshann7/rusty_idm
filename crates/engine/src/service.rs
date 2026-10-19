use crate::{
    events::{Event, EventBus},
    transfer::{self, ProgressSink, TransferControl, TransferOutcome, TransferProgress},
    *,
};
use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    future::Future,
    pin::Pin,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant, SystemTime},
};
use tokio::{
    sync::mpsc,
    task::{Id, JoinSet},
};
use tokio_util::sync::CancellationToken;

pub enum Command {
    Submit(DownloadRequest),
    Start(JobId),
    Cancel(JobId),
    Pause(JobId),
    Resume(JobId),
    SetSpeedLimit(JobId, Option<u64>),
    CancelAll,
    Remove(JobId),
    Retry(JobId),
    Shutdown,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandError {
    QueueFull,
    Closed,
}
impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::QueueFull => "The command queue is full. Try again after the next update.",
            Self::Closed => "The download engine is closing or unavailable.",
        })
    }
}
impl std::error::Error for CommandError {}
#[derive(Debug)]
pub struct EngineJoinError;
impl std::fmt::Display for EngineJoinError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("The download engine did not shut down normally.")
    }
}
impl std::error::Error for EngineJoinError {}

pub type TransferFuture = Pin<Box<dyn Future<Output = Result<TransferOutcome, JobError>> + Send>>;
pub trait TransferExecutor: Send + Sync + 'static {
    fn execute(
        &self,
        request: DownloadRequest,
        cancel: TransferControl,
        progress: ProgressSink,
    ) -> TransferFuture;
}
struct HttpExecutor;
impl TransferExecutor for HttpExecutor {
    fn execute(
        &self,
        request: DownloadRequest,
        cancel: TransferControl,
        progress: ProgressSink,
    ) -> TransferFuture {
        Box::pin(transfer::controlled_transfer(request, cancel, progress))
    }
}

pub struct EngineHandle {
    sender: mpsc::Sender<Command>,
    bus: Arc<EventBus>,
    closing: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}
impl EngineHandle {
    pub fn try_send(&self, command: Command) -> Result<(), CommandError> {
        if self.closing.load(Ordering::Acquire) {
            return Err(CommandError::Closed);
        }
        let shutdown = matches!(command, Command::Shutdown);
        self.sender.try_send(command).map_err(|e| match e {
            mpsc::error::TrySendError::Full(_) => CommandError::QueueFull,
            mpsc::error::TrySendError::Closed(_) => CommandError::Closed,
        })?;
        if shutdown {
            self.closing.store(true, Ordering::Release);
        }
        Ok(())
    }
    pub fn drain_events(&self) -> Vec<Event> {
        self.bus.drain()
    }
    /// Call only after ShutdownComplete, outside window callbacks.
    pub fn join(self) -> Result<(), EngineJoinError> {
        let Self {
            sender,
            closing,
            thread,
            ..
        } = self;
        closing.store(true, Ordering::Release);
        drop(sender);
        thread
            .ok_or(EngineJoinError)?
            .join()
            .map_err(|_| EngineJoinError)
    }
}
// Dropping all senders closes the channel and initiates cleanup; Drop never joins.
pub fn spawn_engine(wake: Arc<dyn Fn() + Send + Sync>) -> EngineHandle {
    spawn_engine_with_executor(wake, Arc::new(HttpExecutor))
}
pub fn spawn_engine_with_executor(
    wake: Arc<dyn Fn() + Send + Sync>,
    executor: Arc<dyn TransferExecutor>,
) -> EngineHandle {
    let (sender, receiver) = mpsc::channel(64);
    let bus = Arc::new(EventBus::new(wake));
    let events = bus.clone();
    let closing = Arc::new(AtomicBool::new(false));
    let flag = closing.clone();
    let thread = thread::spawn(move || {
        match tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
        {
            Ok(runtime) => runtime.block_on(actor(receiver, events, flag, executor)),
            Err(_) => {
                flag.store(true, Ordering::Release);
                events.publish(Event::CommandRejected {
                    message: "Cannot start the download runtime.".into(),
                });
                events.publish(Event::ShutdownComplete);
            }
        }
    });
    EngineHandle {
        sender,
        bus,
        closing,
        thread: Some(thread),
    }
}

struct Job {
    request: DownloadRequest,
    snapshot: JobSnapshot,
    cancel: TransferControl,
    remove: bool,
    sample: (Instant, u64),
    started: bool,
}
type ProgressMap = Arc<Mutex<HashMap<JobId, TransferProgress>>>;
async fn actor(
    mut receiver: mpsc::Receiver<Command>,
    bus: Arc<EventBus>,
    closing: Arc<AtomicBool>,
    executor: Arc<dyn TransferExecutor>,
) {
    let mut jobs: BTreeMap<JobId, Job> = BTreeMap::new();
    let mut waiting = VecDeque::new();
    let mut tasks = JoinSet::new();
    let mut task_jobs: HashMap<Id, JobId> = HashMap::new();
    let progress: ProgressMap = Arc::new(Mutex::new(HashMap::new()));
    let mut next_id = 1u64;
    let mut shutdown = false;
    let mut tick = tokio::time::interval(Duration::from_millis(100));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        if shutdown && tasks.is_empty() {
            bus.publish(Event::ShutdownComplete);
            break;
        }
        tokio::select! {
            biased;
            command = receiver.recv(), if !shutdown => {
                match command {
                    None | Some(Command::Shutdown) => { shutdown = true; closing.store(true, Ordering::Release); receiver.close(); for job in jobs.values_mut() { cancel_job(job, &bus); } waiting.clear(); },
                    Some(Command::Submit(request)) => {
                        if closing.load(Ordering::Acquire) { reject(&bus, "The engine is shutting down."); continue; }
                        if jobs.len() + bus.removed_pending() >= 1000 { reject(&bus, "The 1,000-entry limit has been reached. Remove inactive entries first."); continue; }
                        let id = JobId(next_id); next_id += 1;
                        let snapshot = JobSnapshot { id, filename: request.destination.file_name().unwrap_or_default().to_string_lossy().into_owned(), source_host: request.source_host().into(), display_url: request.display_url(), destination: request.destination.clone(), state: JobState::Waiting, received: 0, total: None, resume_capable: false, speed_limit: None, bytes_per_second: 0, eta_seconds: None, last_try: None, error: None, deferred: request.deferred, category: request.category.clone(), description: request.description.clone() };
                        let deferred = request.deferred;
                        bus.publish(Event::JobChanged(snapshot.clone()));
                        jobs.insert(id, Job { request, snapshot, cancel: TransferControl::new(CancellationToken::new()), remove: false, started: false, sample: (Instant::now(), 0) }); if !deferred { waiting.push_back(id); }
                    },
                    Some(Command::Start(id)) => {
                        if closing.load(Ordering::Acquire) { reject(&bus, "The engine is shutting down."); continue; }
                        if let Some(job) = jobs.get_mut(&id) {
                            if job.snapshot.state != JobState::Waiting || !job.snapshot.deferred { reject(&bus, "Only a download saved for later can be started."); continue; }
                            job.snapshot.deferred = false; job.request.deferred = false;
                            waiting.push_back(id); bus.publish(Event::JobChanged(job.snapshot.clone()));
                        }
                    },
                    Some(Command::Cancel(id)) => { if let Some(job) = jobs.get_mut(&id) { cancel_job(job, &bus); } },
                    Some(Command::Pause(id)) => {
                        if let Some(job) = jobs.get_mut(&id) {
                            if job.snapshot.state == JobState::Running && job.cancel.try_pause() {
                                job.snapshot.state = JobState::Pausing;
                                job.snapshot.bytes_per_second = 0; job.snapshot.eta_seconds = None;
                                bus.publish(Event::JobChanged(job.snapshot.clone()));
                            }
                        }
                    },
                    Some(Command::Resume(id)) => {
                        if let Some(job) = jobs.get_mut(&id) {
                            if job.snapshot.state == JobState::Paused {
                                job.snapshot.state = JobState::Waiting;
                                waiting.push_back(id);
                                bus.publish(Event::JobChanged(job.snapshot.clone()));
                            }
                        }
                    },
                    Some(Command::SetSpeedLimit(id, limit)) => {
                        if limit == Some(0) { reject(&bus, "Enter a positive speed limit, or turn the limiter off."); continue; }
                        if let Some(job) = jobs.get_mut(&id) {
                            if !job.snapshot.state.is_terminal() && job.snapshot.state != JobState::Cancelling {
                                job.cancel.set_speed_limit(limit); job.snapshot.speed_limit = limit;
                                bus.publish(Event::JobChanged(job.snapshot.clone()));
                            }
                        }
                    },
                    Some(Command::CancelAll) => { for job in jobs.values_mut() { cancel_job(job, &bus); } },
                    Some(Command::Remove(id)) => {
                        if jobs.get(&id).is_some_and(|job| job.snapshot.state.is_terminal() && has_retained_partial(job)) && bus.terminal_pending(id) { reject(&bus, "A partial file needs attention. Read its recovery path before removing the entry."); continue; }
                        if let Some(job) = jobs.get_mut(&id) { if job.snapshot.state.is_terminal() { jobs.remove(&id); bus.publish(Event::JobRemoved(id)); } else { job.remove = true; cancel_job(job, &bus); if job.snapshot.state.is_terminal() { jobs.remove(&id); bus.publish(Event::JobRemoved(id)); } } }
                    },
                    Some(Command::Retry(id)) => {
                        if let Some(job) = jobs.get_mut(&id) {
                            if !job.snapshot.state.can_retry() { reject(&bus, "Only failed or cancelled entries can be retried."); continue; }
                            if has_retained_partial(job) { reject(&bus, "A retained partial needs attention. Use Add URL with a new filename; this version cannot resume retained bytes."); continue; }
                            if bus.terminal_pending(id) { reject(&bus, "Wait for the previous result to appear before retrying."); continue; }
                            job.snapshot.received = 0; job.snapshot.total = None; job.snapshot.error = None; job.snapshot.last_try = None;
                            job.snapshot.bytes_per_second = 0; job.snapshot.eta_seconds = None;
                            job.snapshot.resume_capable = false; job.started = false;
                            job.cancel = TransferControl::new(CancellationToken::new()); job.remove = false;
                            job.cancel.set_speed_limit(job.snapshot.speed_limit);
                            // Old approval is never reused on retry. prepare() validates again.
                            job.request.replacement = ReplacementPolicy::Deny;
                            job.snapshot.deferred = false; job.request.deferred = false;
                            job.snapshot.state = JobState::Waiting; waiting.push_back(id); bus.publish(Event::JobChanged(job.snapshot.clone()));
                        }
                    },
                }
            },
            result = tasks.join_next_with_id(), if !tasks.is_empty() => {
                let (task_id, result) = match result.expect("nonempty tasks") { Ok((id, result)) => (id, result), Err(error) => (error.id(), Err(JobError::new(ErrorKind::Io, "The transfer worker stopped unexpectedly."))) };
                let Some(id) = task_jobs.remove(&task_id) else { continue; };
                if let Some(job) = jobs.get_mut(&id) {
                    if let Some(p) = progress.lock().unwrap_or_else(|e| e.into_inner()).remove(&id) { job.snapshot.received = p.received; job.snapshot.total = p.total; job.snapshot.resume_capable = p.resume_capable; }
                    match result {
                        Ok(TransferOutcome::Completed { bytes }) if job.snapshot.state != JobState::Cancelling => { job.snapshot.received = bytes; job.snapshot.total = Some(bytes); job.snapshot.state = JobState::Completed; },
                        Ok(TransferOutcome::Completed { .. }) => job.snapshot.state = JobState::Cancelled,
                        Ok(TransferOutcome::Cancelled) => job.snapshot.state = JobState::Cancelled,
                        Err(error) => { job.snapshot.state = JobState::Failed; job.snapshot.error = Some(error); },
                    }
                    job.snapshot.bytes_per_second = 0; job.snapshot.eta_seconds = None;
                    bus.publish(Event::JobChanged(job.snapshot.clone()));
                    if job.remove && !has_retained_partial(job) { jobs.remove(&id); bus.publish(Event::JobRemoved(id)); }
                    else { job.remove = false; }
                }
            },
            _ = tick.tick() => {
                let updates = std::mem::take(&mut *progress.lock().unwrap_or_else(|p| p.into_inner()));
                for (id, job) in jobs.iter_mut() {
                    if !matches!(job.snapshot.state, JobState::Running | JobState::Pausing) { continue; }
                    let update = updates.get(id).copied().unwrap_or(TransferProgress { received: job.snapshot.received, total: job.snapshot.total, resume_capable: job.snapshot.resume_capable, paused: false });
                    if job.snapshot.state == JobState::Pausing && update.paused { job.snapshot.state = JobState::Paused; }
                    let elapsed = job.sample.0.elapsed().as_secs_f64().max(0.001);
                    job.snapshot.received = update.received; job.snapshot.total = update.total;
                    job.snapshot.resume_capable = update.resume_capable;
                    job.snapshot.bytes_per_second = if job.snapshot.state == JobState::Running { ((update.received.saturating_sub(job.sample.1)) as f64 / elapsed) as u64 } else { 0 };
                    job.snapshot.eta_seconds = update.total.and_then(|t| t.checked_sub(update.received)).and_then(|remaining| remaining.checked_div(job.snapshot.bytes_per_second));
                    job.sample = (Instant::now(), update.received); bus.publish(Event::JobChanged(job.snapshot.clone()));
                }
            },
        }
        // Queued cancellations cannot admit work; cleanup must release the task slot.
        while !shutdown
            && !closing.load(Ordering::Acquire)
            && jobs
                .values()
                .filter(|job| {
                    matches!(
                        job.snapshot.state,
                        JobState::Running | JobState::Pausing | JobState::Cancelling
                    )
                })
                .count()
                < 2
        {
            let Some(id) = waiting.pop_front() else {
                break;
            };
            let Some(job) = jobs.get_mut(&id) else {
                continue;
            };
            if job.snapshot.state != JobState::Waiting {
                continue;
            }
            job.snapshot.state = JobState::Running;
            if !job.started {
                job.snapshot.last_try = Some(SystemTime::now());
            }
            job.sample = (Instant::now(), job.snapshot.received);
            bus.publish(Event::JobChanged(job.snapshot.clone()));
            if job.started {
                job.cancel.try_resume();
                continue;
            }
            job.started = true;
            let updates = progress.clone();
            let sink: ProgressSink = Arc::new(move |p| {
                updates
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .insert(id, p);
            });
            let future = executor.execute(job.request.clone(), job.cancel.clone(), sink);
            let task = tasks.spawn(future);
            task_jobs.insert(task.id(), id);
        }
    }
}
fn reject(bus: &EventBus, message: &str) {
    bus.publish(Event::CommandRejected {
        message: message.into(),
    });
}
fn has_retained_partial(job: &Job) -> bool {
    job.snapshot
        .error
        .as_ref()
        .is_some_and(|e| e.retained_partial.is_some() || e.retained_original.is_some())
}
fn cancel_job(job: &mut Job, bus: &EventBus) {
    match job.snapshot.state {
        JobState::Waiting if !job.started => job.snapshot.state = JobState::Cancelled,
        JobState::Waiting | JobState::Running | JobState::Pausing | JobState::Paused => {
            if !job.cancel.try_cancel() {
                return;
            }
            job.snapshot.state = JobState::Cancelling;
        }
        _ => return,
    }
    job.snapshot.bytes_per_second = 0;
    job.snapshot.eta_seconds = None;
    bus.publish(Event::JobChanged(job.snapshot.clone()));
}
