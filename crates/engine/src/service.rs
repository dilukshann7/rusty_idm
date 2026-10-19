use crate::submissions::*;
use crate::{
    events::{Event, EventBus},
    transfer::{self, ProgressSink, TransferControl, TransferOutcome, TransferProgress},
    *,
};
use rusty_browser_bridge::{
    lease::LeaseFailure,
    protocol::{RequestId, MAX_RECEIPTS, RETENTION},
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
    External(ExternalCommand),
    Submit(DownloadRequest),
    Start(JobId),
    Cancel(JobId),
    Pause(JobId),
    Resume(JobId),
    Activate(JobId),
    Restart(JobId),
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
#[derive(Clone)]
pub struct EngineClient {
    sender: mpsc::Sender<Command>,
    closing: Arc<AtomicBool>,
}
impl EngineClient {
    /// Nonblocking session-end signal; also works while GUI modal loops are open.
    pub fn request_shutdown(&self) {
        self.closing.store(true, Ordering::Release);
    }
    pub fn try_external(&self, command: ExternalCommand) -> Result<(), CommandError> {
        if self.closing.load(Ordering::Acquire) {
            return Err(CommandError::Closed);
        }
        self.sender
            .try_send(Command::External(command))
            .map_err(|e| match e {
                mpsc::error::TrySendError::Full(_) => CommandError::QueueFull,
                mpsc::error::TrySendError::Closed(_) => CommandError::Closed,
            })
    }
}
impl EngineHandle {
    pub fn client(&self) -> EngineClient {
        EngineClient {
            sender: self.sender.clone(),
            closing: self.closing.clone(),
        }
    }
    pub fn try_external(&self, command: ExternalCommand) -> Result<(), CommandError> {
        self.client().try_external(command)
    }
    pub fn find_duplicate(&self, request: &DownloadRequest) -> Option<JobSnapshot> {
        self.bus.find_duplicate(request)
    }
    pub fn destinations(&self) -> Vec<std::path::PathBuf> {
        self.bus.destinations()
    }
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
    restart: bool,
    resume_after_pause: bool,
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
    let mut receipts: HashMap<RequestId, Record> = HashMap::new();
    let mut shutdown = false;
    let mut tick = tokio::time::interval(Duration::from_millis(100));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        // The GUI's WM_ENDSESSION callback can only signal, never borrow its
        // AppState or join this thread. Observe that signal independently of
        // the bounded command queue; the tick bounds an idle actor's wakeup.
        if !shutdown && closing.load(Ordering::Acquire) {
            shutdown = true;
            receiver.close();
            for job in jobs.values_mut() {
                job.restart = false;
                job.resume_after_pause = false;
                cancel_job(job, &bus);
            }
            waiting.clear();
        }
        if shutdown && tasks.is_empty() {
            bus.publish(Event::ShutdownComplete);
            break;
        }
        tokio::select! {
            biased;
            command = receiver.recv(), if !shutdown => {
                match command {
                    Some(Command::External(command)) => {
                        receipts.retain(|_,r|r.at.elapsed()<RETENTION);
                        match command {
                            ExternalCommand::Apply{id,action,lease,reply}=> {
                                if let Some(record)=receipts.get(&id) { let _=reply.send(if record.action.same(&action) { record.result.clone() } else { Err(SubmissionError::InvalidAction) }); continue; }
                                if receipts.len()>=MAX_RECEIPTS { let _=reply.send(Err(SubmissionError::Busy)); continue; }
                                let result=lease.try_accept(|| {
                                    if closing.load(Ordering::Acquire) { return Err(SubmissionError::Closing); }
                                    let (job_id,created_new)=match &action {
                                        SubmissionAction::Submit(request)=>(admit(request.clone(),&mut jobs,&mut waiting,&mut next_id,&bus)?,true),
                                        SubmissionAction::Show(id)=> { if !jobs.contains_key(id) { return Err(SubmissionError::NotFound); } (*id,false) },
                                        SubmissionAction::Activate(id)=> { let job=jobs.get_mut(id).ok_or(SubmissionError::NotFound)?; activate_external(job,&mut waiting,&bus)?; (*id,false) },
                                        SubmissionAction::Restart(id)=> {
                                            let job=jobs.get(id).ok_or(SubmissionError::NotFound)?;
                                            if has_retained_partial(job) { return Err(SubmissionError::RecoveryRequired); }
                                            if jobs.iter().any(|(other_id,other)|other_id!=id && other.snapshot.state.is_active() && same_destination(&job.request.destination,&other.request.destination)) { return Err(SubmissionError::InvalidAction); }
                                            let job=jobs.get_mut(id).expect("checked");
                                            // Approval inspection happens before acknowledging this action.
                                            let identity=inspect_destination(&job.request.destination).map_err(|_|SubmissionError::InvalidAction)?;
                                            job.request.replacement=identity.map(ReplacementPolicy::Approved).unwrap_or(ReplacementPolicy::Deny);
                                            if job.snapshot.state.is_terminal() && !bus.terminal_pending(*id) || job.snapshot.state==JobState::Waiting && !job.started {
                                                let replacement=job.request.replacement.clone(); reset_job(job,replacement); waiting.retain(|j|j!=id); waiting.push_back(*id); bus.publish(Event::JobChanged(job.snapshot.clone()));
                                            } else { job.restart=true; job.resume_after_pause=false; cancel_job(job,&bus); }
                                            (*id,false)
                                        }
                                    };
                                    Ok(SubmissionReceipt{id:id.clone(),job_id,created_new})
                                }).map_err(|e|match e { LeaseFailure::Expired=>SubmissionError::Expired,LeaseFailure::Revoked=>SubmissionError::Revoked,LeaseFailure::AlreadyAccepted=>SubmissionError::InvalidAction,LeaseFailure::Action(e)=>e });
                                receipts.insert(id,Record{action,result:result.clone(),cancelled:false,at:Instant::now()}); let _=reply.send(result);
                            },
                            ExternalCommand::Status{id,reply}=> { let status=receipts.get(&id).map(|r| if r.cancelled { SubmissionStatus::Declined(SubmissionError::Revoked) } else { match &r.result { Ok(receipt)=>SubmissionStatus::Accepted(receipt.clone()),Err(e)=>SubmissionStatus::Declined(*e) } }).unwrap_or(SubmissionStatus::Unknown); let _=reply.send(status); },
                            ExternalCommand::CancelOwned{id,reply}=> {
                                let result=match receipts.get_mut(&id) {
                                    Some(record)=>match &record.result {
                                        Ok(receipt) if !receipt.created_new=>CancelOwnedResult::NotOwned,
                                        Ok(_) if record.cancelled=>CancelOwnedResult::Cancelled,
                                        Ok(receipt)=>match jobs.get_mut(&receipt.job_id) {
                                            Some(job) if !job.snapshot.state.is_terminal()=>{ job.restart=false; job.resume_after_pause=false; cancel_job(job,&bus); record.cancelled=true; CancelOwnedResult::Cancelled },
                                            _=>CancelOwnedResult::AlreadyFinished,
                                        }, _=>CancelOwnedResult::Unknown }, None=>CancelOwnedResult::Unknown }; let _=reply.send(result);
                            }
                        }
                    },
                    None | Some(Command::Shutdown) => { shutdown = true; closing.store(true, Ordering::Release); receiver.close(); for job in jobs.values_mut() { job.restart = false; job.resume_after_pause = false; cancel_job(job, &bus); } waiting.clear(); },
                    Some(Command::Submit(request)) => {
                        if closing.load(Ordering::Acquire) { reject(&bus, "The engine is shutting down."); continue; }
                        if admit(request,&mut jobs,&mut waiting,&mut next_id,&bus).is_err() { reject(&bus,"Download admission rejected. Check duplicates, entry limits and save path."); }
                    },
                    Some(Command::Start(id)) => {
                        if closing.load(Ordering::Acquire) { reject(&bus, "The engine is shutting down."); continue; }
                        if let Some(job) = jobs.get_mut(&id) {
                            if job.snapshot.state != JobState::Waiting || !job.snapshot.deferred { reject(&bus, "Only a download saved for later can be started."); continue; }
                            job.snapshot.deferred = false; job.request.deferred = false;
                            waiting.push_back(id); bus.publish(Event::JobChanged(job.snapshot.clone()));
                        }
                    },
                    Some(Command::Cancel(id)) => { if let Some(job) = jobs.get_mut(&id) { job.restart = false; job.resume_after_pause = false; cancel_job(job, &bus); } },
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
                    Some(Command::Activate(id)) => {
                        if let Some(job) = jobs.get_mut(&id) {
                            match job.snapshot.state {
                                JobState::Paused => { job.snapshot.state = JobState::Waiting; waiting.push_back(id); },
                                JobState::Pausing => job.resume_after_pause = true,
                                JobState::Waiting if job.snapshot.deferred => { job.snapshot.deferred = false; job.request.deferred = false; waiting.push_back(id); },
                                state if state.can_retry() => {
                                    if has_retained_partial(job) { reject(&bus, "This entry has retained recovery files. Copy its recovery paths and use a numbered copy."); continue; }
                                    if bus.terminal_pending(id) { reject(&bus, "Wait for the previous result before resuming this entry."); continue; }
                                    reset_job(job, ReplacementPolicy::Deny); waiting.push_back(id);
                                },
                                _ => {},
                            }
                            bus.publish(Event::JobChanged(job.snapshot.clone()));
                        }
                    },
                    Some(Command::Restart(id)) => {
                        let conflict = jobs.get(&id).is_some_and(|job| jobs.iter().any(|(other_id, other)| *other_id != id && other.snapshot.state.is_active() && same_destination(&job.request.destination, &other.request.destination)));
                        if conflict { reject(&bus, "Another active download uses that filename. Stop it or choose a numbered copy."); continue; }
                        if let Some(job) = jobs.get_mut(&id) {
                            if has_retained_partial(job) { reject(&bus, "This entry has recovery files. Copy its recovery paths before restarting."); continue; }
                            if job.snapshot.state.is_terminal() {
                                if bus.terminal_pending(id) { job.restart = true; continue; }
                                if restart_job(job, &bus) { waiting.push_back(id); }
                            } else {
                                if job.snapshot.state == JobState::Waiting && !job.started {
                                    if restart_job(job, &bus) { waiting.retain(|queued| *queued != id); waiting.push_back(id); }
                                    continue;
                                }
                                job.restart = true; job.resume_after_pause = false;
                                cancel_job(job, &bus);
                                if job.snapshot.state.is_terminal() && restart_job(job, &bus) { waiting.push_back(id); }
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
                    Some(Command::CancelAll) => { for job in jobs.values_mut() { job.restart = false; job.resume_after_pause = false; cancel_job(job, &bus); } },
                    Some(Command::Remove(id)) => {
                        if jobs.get(&id).is_some_and(|job| job.snapshot.state.is_terminal() && has_retained_partial(job)) && bus.terminal_pending(id) { reject(&bus, "A partial file needs attention. Read its recovery path before removing the entry."); continue; }
                        if let Some(job) = jobs.get_mut(&id) { job.restart = false; job.resume_after_pause = false; if job.snapshot.state.is_terminal() { jobs.remove(&id); bus.publish(Event::JobRemoved(id)); } else { job.remove = true; cancel_job(job, &bus); if job.snapshot.state.is_terminal() { jobs.remove(&id); bus.publish(Event::JobRemoved(id)); } } }
                    },
                    Some(Command::Retry(id)) => {
                        if let Some(job) = jobs.get_mut(&id) {
                            if !job.snapshot.state.can_retry() { reject(&bus, "Only failed or cancelled entries can be retried."); continue; }
                            if has_retained_partial(job) { reject(&bus, "A retained partial needs attention. Use Add URL with a new filename; this version cannot resume retained bytes."); continue; }
                            if bus.terminal_pending(id) { reject(&bus, "Wait for the previous result to appear before retrying."); continue; }
                            // Old approval is never reused on retry. prepare() validates again.
                            reset_job(job, ReplacementPolicy::Deny);
                            waiting.push_back(id); bus.publish(Event::JobChanged(job.snapshot.clone()));
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
                    if job.restart && !shutdown && !closing.load(Ordering::Acquire) && !has_retained_partial(job) {
                        // Restart is serialized after cleanup, including a late
                        // completion that beat cancellation. No shared partial.
                        if restart_job(job, &bus) { waiting.push_back(id); }
                        continue;
                    }
                    job.restart = false;
                    bus.publish(Event::JobChanged(job.snapshot.clone()));
                    if job.remove && !has_retained_partial(job) { jobs.remove(&id); bus.publish(Event::JobRemoved(id)); }
                    else { job.remove = false; }
                }
            },
            _ = tick.tick() => {
                let updates = std::mem::take(&mut *progress.lock().unwrap_or_else(|p| p.into_inner()));
                for (id, job) in jobs.iter_mut() {
                    if job.restart && job.snapshot.state.is_terminal() && !bus.terminal_pending(*id) && !has_retained_partial(job) && !shutdown && !closing.load(Ordering::Acquire) {
                        if restart_job(job, &bus) { waiting.push_back(*id); }
                        continue;
                    }
                    if !matches!(job.snapshot.state, JobState::Running | JobState::Pausing) { continue; }
                    let update = updates.get(id).copied().unwrap_or(TransferProgress { received: job.snapshot.received, total: job.snapshot.total, resume_capable: job.snapshot.resume_capable, paused: false });
                    if job.snapshot.state == JobState::Pausing && update.paused {
                        job.snapshot.state = JobState::Paused;
                        if job.resume_after_pause { job.resume_after_pause = false; job.snapshot.state = JobState::Waiting; waiting.push_back(*id); }
                    }
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
fn same_destination(a: &std::path::Path, b: &std::path::Path) -> bool {
    a.to_string_lossy().replace('/', "\\").to_lowercase()
        == b.to_string_lossy().replace('/', "\\").to_lowercase()
}
fn reset_job(job: &mut Job, replacement: ReplacementPolicy) {
    job.snapshot.state = JobState::Waiting;
    job.snapshot.received = 0;
    job.snapshot.total = None;
    job.snapshot.error = None;
    job.snapshot.last_try = None;
    job.snapshot.resume_capable = false;
    job.snapshot.bytes_per_second = 0;
    job.snapshot.eta_seconds = None;
    job.snapshot.deferred = false;
    job.request.deferred = false;
    job.request.replacement = replacement;
    job.cancel = TransferControl::new(CancellationToken::new());
    job.cancel.set_speed_limit(job.snapshot.speed_limit);
    job.started = false;
    job.restart = false;
    job.resume_after_pause = false;
    job.remove = false;
}
fn restart_job(job: &mut Job, bus: &EventBus) -> bool {
    job.restart = false;
    match inspect_destination(&job.request.destination) {
        Ok(identity) => {
            reset_job(
                job,
                identity
                    .map(ReplacementPolicy::Approved)
                    .unwrap_or(ReplacementPolicy::Deny),
            );
            bus.publish(Event::JobChanged(job.snapshot.clone()));
            true
        }
        Err(error) => {
            job.snapshot.state = JobState::Failed;
            job.snapshot.error = Some(error);
            bus.publish(Event::JobChanged(job.snapshot.clone()));
            false
        }
    }
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

fn admit(
    request: DownloadRequest,
    jobs: &mut BTreeMap<JobId, Job>,
    waiting: &mut VecDeque<JobId>,
    next_id: &mut u64,
    bus: &EventBus,
) -> Result<JobId, SubmissionError> {
    if jobs.len() + bus.removed_pending() >= 1000 {
        return Err(SubmissionError::InvalidAction);
    }
    if !request.allow_duplicate && jobs.values().any(|job| job.request.same_source(&request)) {
        return Err(SubmissionError::InvalidAction);
    }
    if jobs.values().any(|job| {
        job.snapshot.state.is_active()
            && same_destination(&job.request.destination, &request.destination)
    }) {
        return Err(SubmissionError::InvalidAction);
    }
    let id = JobId(*next_id);
    *next_id += 1;
    bus.register_request(id, &request);
    let snapshot = JobSnapshot {
        id,
        filename: request
            .destination
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
        source_host: request.source_host().into(),
        display_url: request.display_url(),
        destination: request.destination.clone(),
        state: JobState::Waiting,
        received: 0,
        total: None,
        resume_capable: false,
        speed_limit: None,
        bytes_per_second: 0,
        eta_seconds: None,
        last_try: None,
        error: None,
        deferred: request.deferred,
        category: request.category.clone(),
        description: request.description.clone(),
    };
    let deferred = request.deferred;
    bus.publish(Event::JobChanged(snapshot.clone()));
    jobs.insert(
        id,
        Job {
            request,
            snapshot,
            cancel: TransferControl::new(CancellationToken::new()),
            remove: false,
            started: false,
            restart: false,
            resume_after_pause: false,
            sample: (Instant::now(), 0),
        },
    );
    if !deferred {
        waiting.push_back(id);
    }
    Ok(id)
}

fn activate_external(
    job: &mut Job,
    waiting: &mut VecDeque<JobId>,
    bus: &EventBus,
) -> Result<(), SubmissionError> {
    match job.snapshot.state {
        JobState::Paused => {
            job.snapshot.state = JobState::Waiting;
            waiting.push_back(job.snapshot.id);
        }
        JobState::Pausing => job.resume_after_pause = true,
        JobState::Waiting if job.snapshot.deferred => {
            job.snapshot.deferred = false;
            job.request.deferred = false;
            waiting.push_back(job.snapshot.id);
        }
        state if state.can_retry() => {
            if has_retained_partial(job) {
                return Err(SubmissionError::RecoveryRequired);
            }
            if bus.terminal_pending(job.snapshot.id) {
                return Err(SubmissionError::Busy);
            }
            reset_job(job, ReplacementPolicy::Deny);
            waiting.push_back(job.snapshot.id);
        }
        JobState::Cancelling => return Err(SubmissionError::Busy),
        _ => {}
    }
    bus.publish(Event::JobChanged(job.snapshot.clone()));
    Ok(())
}

#[cfg(test)]
mod external_queue_tests {
    use super::*;
    #[test]
    fn queued_command_is_not_an_acknowledgement_and_full_or_closed_queues_reject() {
        let (sender, receiver) = mpsc::channel(1);
        let client = EngineClient {
            sender,
            closing: Arc::new(AtomicBool::new(false)),
        };
        let id = RequestId::parse("12345678-1234-4234-8234-123456789abc").unwrap();
        let (reply, mut result) = tokio::sync::oneshot::channel();
        client
            .try_external(ExternalCommand::Status {
                id: id.clone(),
                reply,
            })
            .unwrap();
        assert_eq!(
            result.try_recv(),
            Err(tokio::sync::oneshot::error::TryRecvError::Empty)
        );
        let (reply, _rx) = tokio::sync::oneshot::channel();
        assert_eq!(
            client.try_external(ExternalCommand::Status {
                id: id.clone(),
                reply
            }),
            Err(CommandError::QueueFull)
        );
        drop(receiver);
        assert_eq!(
            result.try_recv(),
            Err(tokio::sync::oneshot::error::TryRecvError::Closed)
        );
        let (reply, _rx) = tokio::sync::oneshot::channel();
        assert_eq!(
            client.try_external(ExternalCommand::Status { id, reply }),
            Err(CommandError::Closed)
        );
    }
}
