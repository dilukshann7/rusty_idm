use crate::submissions::*;
use crate::{
    events::{Event, EventBus},
    transfer::{self, ProgressSink, TransferControl, TransferOutcome, TransferProgress},
    *,
};
use rusty_browser_bridge::{
    lease::{CaptureLease, LeaseFailure, LeaseState},
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

#[path = "service_queues.rs"]
mod queue_support;
use queue_support::QueueRuntime;
#[path = "service_speed.rs"]
mod speed;
use speed::SpeedMeter;
#[path = "service_refresh.rs"]
mod refresh_support;

pub enum Command {
    PrepareRefresh(
        JobId,
        std::sync::mpsc::SyncSender<Result<Option<String>, String>>,
    ),
    RefreshAddress {
        job_id: JobId,
        candidate: crate::refresh::RefreshCandidate,
        reply: tokio::sync::oneshot::Sender<Result<(), String>>,
    },
    CreateQueue(String),
    CreateQueueForEditor {
        request: u64,
        name: String,
    },
    ConfigureQueue {
        request: u64,
        id: QueueId,
        settings: QueueSettings,
    },
    RenameQueue(QueueId, String),
    DeleteQueue(QueueId),
    StartQueue(QueueId),
    StopQueue(QueueId),
    QueueParallelism(QueueId, u8),
    AssignQueue(JobId, Option<QueueId>),
    MoveQueueEntry(JobId, bool),
    SupplyCredentials(JobId, String, String),
    ConfirmRestart(JobId),
    SetCategory(JobId, Option<String>),
    External(ExternalCommand),
    Submit(DownloadRequest),
    ConfirmInfo(JobId),
    Inspect(
        JobId,
        tokio::sync::oneshot::Sender<Option<(JobSnapshot, bool)>>,
    ),
    QueryQueues(tokio::sync::oneshot::Sender<Vec<QueueSnapshot>>),
    CreateQueueReply(
        String,
        tokio::sync::oneshot::Sender<Result<QueueId, String>>,
    ),
    RenameCategories(
        Vec<(String, String)>,
        tokio::sync::oneshot::Sender<Result<CategoryRenameUndo, String>>,
    ),
    RestoreCategories(
        CategoryRenameUndo,
        tokio::sync::oneshot::Sender<Result<(), String>>,
    ),
    Start(JobId),
    Cancel(JobId),
    Pause(JobId),
    Resume(JobId),
    Activate(JobId),
    Restart(JobId),
    SetSpeedLimit(JobId, Option<u64>),
    CancelAll,
    Remove(JobId),
    RemoveCompleted,
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
        self.client().try_send(command)
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
    spawn(wake, Arc::new(HttpExecutor), true)
}
/// Start the production transfer executor with an isolated in-memory queue store.
/// Useful for integration tests that must not read or modify the user's queue data.
pub fn spawn_engine_in_memory(wake: Arc<dyn Fn() + Send + Sync>) -> EngineHandle {
    spawn(wake, Arc::new(HttpExecutor), false)
}
pub fn spawn_engine_with_executor(
    wake: Arc<dyn Fn() + Send + Sync>,
    executor: Arc<dyn TransferExecutor>,
) -> EngineHandle {
    spawn(wake, executor, false)
}
fn spawn(
    wake: Arc<dyn Fn() + Send + Sync>,
    executor: Arc<dyn TransferExecutor>,
    durable: bool,
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
            Ok(runtime) => runtime.block_on(actor(receiver, events, flag, executor, durable)),
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
    speed_meter: SpeedMeter,
    started: bool,
    restart: bool,
    resume_after_pause: bool,
    user_held: bool,
}
enum RefreshReply {
    Manual(tokio::sync::oneshot::Sender<Result<(), String>>),
    Browser {
        id: RequestId,
        lease: CaptureLease,
        reply: tokio::sync::oneshot::Sender<Result<SubmissionReceipt, SubmissionError>>,
    },
}
struct PendingRefresh {
    checkpoint: crate::storage::Checkpoint,
    candidate: crate::refresh::RefreshCandidate,
    reply: RefreshReply,
}
type ProgressMap = Arc<Mutex<HashMap<JobId, TransferProgress>>>;
async fn actor(
    mut receiver: mpsc::Receiver<Command>,
    bus: Arc<EventBus>,
    closing: Arc<AtomicBool>,
    executor: Arc<dyn TransferExecutor>,
    durable: bool,
) {
    let mut jobs: BTreeMap<JobId, Job> = BTreeMap::new();
    let mut waiting = VecDeque::new();
    let mut tasks = JoinSet::new();
    let mut task_jobs: HashMap<Id, JobId> = HashMap::new();
    let mut refresh_tasks: JoinSet<(JobId, Result<crate::refresh::VerifiedRefresh, JobError>)> =
        JoinSet::new();
    let mut refresh_task_jobs: HashMap<Id, JobId> = HashMap::new();
    let mut pending_refreshes: HashMap<JobId, PendingRefresh> = HashMap::new();
    let mut prepare_replies: HashMap<
        JobId,
        std::sync::mpsc::SyncSender<Result<Option<String>, String>>,
    > = HashMap::new();
    let progress: ProgressMap = Arc::new(Mutex::new(HashMap::new()));
    let mut next_id = 1u64;
    let mut queues = QueueRuntime::new(durable);
    queues.restore(&mut jobs, &mut next_id, &bus);
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
                shutdown_job(job, &bus);
            }
            waiting.clear();
        }
        if shutdown && tasks.is_empty() {
            refresh_tasks.abort_all();
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
                            ExternalCommand::Refresh { id, job_id, candidate, lease, reply } => {
                                if let Some(record) = receipts.get(&id) {
                                    let result = if record.action.same(&SubmissionAction::Refresh(job_id)) { record.result.clone() } else { Err(SubmissionError::InvalidAction) };
                                    let _ = reply.send(result);
                                    continue;
                                }
                                if lease.state() != LeaseState::Pending || receipts.len() >= MAX_RECEIPTS {
                                    let _ = reply.send(Err(SubmissionError::Busy));
                                    continue;
                                }
                                match refresh_support::preflight(&jobs, &pending_refreshes, job_id, &candidate) {
                                    Ok((checkpoint, ignore_modified)) => {
                                        let verify_checkpoint = checkpoint.clone();
                                        let verify_candidate = candidate.clone();
                                        let task = refresh_tasks.spawn(async move { (job_id, crate::refresh::verify_checkpoint(verify_checkpoint, ignore_modified, verify_candidate).await) });
                                        refresh_task_jobs.insert(task.id(), job_id);
                                        pending_refreshes.insert(job_id, PendingRefresh {
                                            checkpoint, candidate,
                                            reply: RefreshReply::Browser { id, lease, reply },
                                        });
                                    }
                                    Err(error) => {
                                        reject(&bus, &error);
                                        let _ = reply.send(Err(SubmissionError::RecoveryRequired));
                                    }
                                }
                            }
                            ExternalCommand::Apply{id,action,lease,reply}=> {
                                if let Some(record)=receipts.get(&id) { let _=reply.send(if record.action.same(&action) { record.result.clone() } else { Err(SubmissionError::InvalidAction) }); continue; }
                                if receipts.len()>=MAX_RECEIPTS { let _=reply.send(Err(SubmissionError::Busy)); continue; }
                                let mut result=lease.try_accept(|| {
                                    if closing.load(Ordering::Acquire) { return Err(SubmissionError::Closing); }
                                    let (job_id,created_new)=match &action {
                                        SubmissionAction::Submit(request)=>(admit(request.clone(),&mut jobs,&mut waiting,&mut next_id,&bus,&queues,true)?,true),
                                        SubmissionAction::Show(id)=> { if !jobs.contains_key(id) { return Err(SubmissionError::NotFound); } (*id,false) },
                                        SubmissionAction::Activate(id)=> { let job=jobs.get_mut(id).ok_or(SubmissionError::NotFound)?; activate_external(job,&mut waiting,&bus)?; (*id,false) },
                                        SubmissionAction::RestartApproved(id,replacement)=>{
                                            let job=jobs.get(id).ok_or(SubmissionError::NotFound)?;
                                            if job.snapshot.readiness!=EntryReadiness::Ready || has_retained_partial(job){return Err(SubmissionError::RecoveryRequired);}
                                            if !(job.snapshot.state.is_terminal()&&!bus.terminal_pending(*id) || job.snapshot.state==JobState::Waiting&&!job.started){return Err(SubmissionError::Busy);}
                                            if jobs.iter().any(|(other,entry)|other!=id&&entry.snapshot.state.is_active()&&same_destination(&job.request.destination,&entry.request.destination)){return Err(SubmissionError::Busy);}
                                            let actual=inspect_destination(&job.request.destination).map_err(|_|SubmissionError::InvalidAction)?;
                                            let approved=match replacement{ReplacementPolicy::Deny=>None,ReplacementPolicy::Approved(identity)=>Some(identity.clone())};
                                            if actual!=approved{return Err(SubmissionError::InvalidAction);}
                                            let job=jobs.get_mut(id).expect("checked");
                                            if let Some(recovery)=&job.request.recovery{
                                                crate::output::discard_checkpoint(recovery).map_err(|_|SubmissionError::RecoveryRequired)?;
                                                job.snapshot.recovery_paths = recovery.store.snapshot().entries.get(&recovery.entry).map(|entry|entry.retained_files.clone()).unwrap_or_default();
                                            }
                                            reset_job(job,replacement.clone());waiting.retain(|j|j!=id);waiting.push_back(*id);bus.publish(Event::JobChanged(job.snapshot.clone()));(*id,false)
                                        },
                                        SubmissionAction::Restart(id)=> {
                                            let job=jobs.get(id).ok_or(SubmissionError::NotFound)?;
                                            if job.snapshot.readiness!=EntryReadiness::Ready || has_retained_partial(job) { return Err(SubmissionError::RecoveryRequired); }
                                            if jobs.iter().any(|(other_id,other)|other_id!=id && other.snapshot.state.is_active() && same_destination(&job.request.destination,&other.request.destination)) { return Err(SubmissionError::InvalidAction); }
                                            let job=jobs.get_mut(id).expect("checked");
                                            // Approval inspection happens before acknowledging this action.
                                            let identity=inspect_destination(&job.request.destination).map_err(|_|SubmissionError::InvalidAction)?;
                                            job.request.replacement=identity.map(ReplacementPolicy::Approved).unwrap_or(ReplacementPolicy::Deny);
                                            if job.snapshot.state.is_terminal() && !bus.terminal_pending(*id) || job.snapshot.state==JobState::Waiting && !job.started {
                                                if let Some(recovery)=&job.request.recovery {
                                                    crate::output::discard_checkpoint(recovery).map_err(|_|SubmissionError::RecoveryRequired)?;
                                                    job.snapshot.recovery_paths = recovery.store.snapshot().entries.get(&recovery.entry).map(|entry|entry.retained_files.clone()).unwrap_or_default();
                                                }
                                                let replacement=job.request.replacement.clone(); reset_job(job,replacement); waiting.retain(|j|j!=id); waiting.push_back(*id); bus.publish(Event::JobChanged(job.snapshot.clone()));
                                            } else { job.restart=true; job.resume_after_pause=false; cancel_job(job,&bus); }
                                            (*id,false)
                                        }
                                        SubmissionAction::Refresh(_) => return Err(SubmissionError::InvalidAction),
                                    };
                                    if lease.deadline_elapsed() {
                                        waiting.retain(|queued|*queued!=job_id);
                                        if created_new {
                                            if let Some(entry)=jobs.get(&job_id).and_then(|j|j.snapshot.entry_id){if let Err(error)=queues.rollback_admission(entry){reject(&bus,&format!("Expired admission is quarantined for recovery: {error}"));}}
                                            jobs.remove(&job_id);bus.publish(Event::JobRemoved(job_id));
                                        }
                                        else if let Some(job)=jobs.get_mut(&job_id) { if job.snapshot.state==JobState::Waiting {job.snapshot.deferred=true;job.request.deferred=true;} }
                                        return Err(SubmissionError::Expired);
                                    }
                                    Ok(SubmissionReceipt{id:id.clone(),job_id,created_new})
                                }).map_err(|e|match e { LeaseFailure::Expired=>SubmissionError::Expired,LeaseFailure::Revoked=>SubmissionError::Revoked,LeaseFailure::AlreadyAccepted=>SubmissionError::InvalidAction,LeaseFailure::Action(e)=>e });
                                // No task can run before this actor iteration ends. Publish the
                                // durable commit only after the lease accepted the pending record.
                                if let Ok(receipt)=&result {
                                    if receipt.created_new {
                                        if let Some(entry)=jobs.get(&receipt.job_id).and_then(|j|j.snapshot.entry_id){
                                            if let Err(error)=queues.finish_admission(entry){
                                                let job_id=receipt.job_id;
                                                waiting.retain(|queued|*queued!=job_id);
                                                let rollback=queues.rollback_admission(entry);
                                                jobs.remove(&job_id);bus.publish(Event::JobRemoved(job_id));
                                                reject(&bus,&format!("Admission was not started: {error}{}",if rollback.is_err(){". Its unacknowledged record remains quarantined for recovery."}else{""}));
                                                result=Err(SubmissionError::RecoveryRequired);
                                            }
                                        }
                                    }
                                }
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
                    None | Some(Command::Shutdown) => { shutdown = true; closing.store(true, Ordering::Release); receiver.close(); for job in jobs.values_mut() { job.restart = false; job.resume_after_pause = false; shutdown_job(job, &bus); } waiting.clear(); },
                    Some(Command::Inspect(id,reply))=>{let _=reply.send(jobs.get(&id).map(|j|(j.snapshot.clone(),j.request.options.confirmation_required)));},
                    Some(Command::PrepareRefresh(id, reply)) => {
                        if prepare_replies.contains_key(&id) {
                            let _ = reply.send(Err("This download is already preparing for refresh.".into()));
                            continue;
                        }
                        match jobs.get_mut(&id) {
                            Some(job) if job.snapshot.state == JobState::Paused && job.started => {
                                job.cancel.preserve_on_exit();
                                job.snapshot.state = JobState::Pausing;
                                bus.publish(Event::JobChanged(job.snapshot.clone()));
                                prepare_replies.insert(id, reply);
                            }
                            Some(job) => {
                                let result = refresh_support::checkpoint(job).map(|_| job.request.source_page.clone());
                                let _ = reply.send(result);
                            }
                            None => { let _ = reply.send(Err("The download no longer exists.".into())); }
                        }
                    },
                    Some(Command::RefreshAddress { job_id, candidate, reply }) => {
                        match refresh_support::preflight(&jobs, &pending_refreshes, job_id, &candidate) {
                            Ok((checkpoint, ignore_modified)) => {
                                let verify_checkpoint = checkpoint.clone();
                                let verify_candidate = candidate.clone();
                                let task = refresh_tasks.spawn(async move { (job_id, crate::refresh::verify_checkpoint(verify_checkpoint, ignore_modified, verify_candidate).await) });
                                refresh_task_jobs.insert(task.id(), job_id);
                                pending_refreshes.insert(job_id, PendingRefresh { checkpoint, candidate, reply: RefreshReply::Manual(reply) });
                            }
                            Err(error) => { reject(&bus, &error); let _ = reply.send(Err(error)); }
                        }
                    },
                    Some(Command::QueryQueues(reply))=>{queues.publish(&jobs,&bus);let _=reply.send(queues.snapshots());},
                    Some(Command::CreateQueueReply(name,reply))=>{let result=queues.create_for_reply(name);queues.publish(&jobs,&bus);let _=reply.send(result);},
                    Some(Command::RenameCategories(renames,reply))=>{let result=queues.rename_categories(&mut jobs,renames,&bus);let _=reply.send(result);},
                    Some(Command::RestoreCategories(undo,reply))=>{let result=queues.restore_categories(&mut jobs,undo,&bus);let _=reply.send(result);},
                    Some(command) if queue_support::handles(&command) => {
                        queues.command(command, &mut jobs, &mut waiting, &bus).await;
                    },
                    Some(Command::Submit(request)) => {
                        if closing.load(Ordering::Acquire) { reject(&bus, "The engine is shutting down."); continue; }
                        if admit(request,&mut jobs,&mut waiting,&mut next_id,&bus,&queues,false).is_err() { reject(&bus,"Download admission rejected. Check duplicates, entry limits and save path."); }
                    },
                    Some(Command::ConfirmInfo(id)) => {
                        if let Some(job)=jobs.get_mut(&id) {
                            if !job.request.options.confirmation_required {continue;}
                            let result=if let Some(r)=&job.request.recovery {r.store.update(|s|{let e=s.entries.get_mut(&r.entry).ok_or("Provisional download was removed.")?;e.options.confirmation_required=false;Ok(())})}else{Err("Provisional admission was not durable.".into())};
                            if let Err(error)=result{reject(&bus,&error);continue;}
                            job.request.options.confirmation_required=false;
                            if job.snapshot.readiness==EntryReadiness::ConfirmationRequired {job.snapshot.readiness=EntryReadiness::Ready;}
                            job.cancel.confirm();
                            if job.snapshot.queue_id.is_none() && matches!(job.snapshot.state,JobState::Paused|JobState::Waiting){job.snapshot.state=JobState::Waiting;job.snapshot.deferred=false;job.request.deferred=false;waiting.push_back(id);}
                            bus.publish(Event::JobChanged(job.snapshot.clone()));
                        }
                    },
                    Some(Command::Start(id)) => {
                        if closing.load(Ordering::Acquire) { reject(&bus, "The engine is shutting down."); continue; }
                        if let Some(job) = jobs.get_mut(&id) {
                            if job.snapshot.state != JobState::Waiting || !job.snapshot.deferred { reject(&bus, "Only a download saved for later can be started."); continue; }
                            job.user_held = false;
                            job.snapshot.deferred = false; job.request.deferred = false;
                            waiting.push_back(id); bus.publish(Event::JobChanged(job.snapshot.clone()));
                        }
                    },
                    Some(Command::Cancel(id)) => { if let Some(job) = jobs.get_mut(&id) { job.restart = false; job.resume_after_pause = false; cancel_job(job, &bus); } },
                    Some(Command::Pause(id)) => {
                        if let Some(job) = jobs.get_mut(&id) {
                            job.user_held = true;
                            if job.snapshot.state == JobState::Waiting {waiting.retain(|queued|*queued!=id);job.snapshot.deferred=true;job.request.deferred=true;bus.publish(Event::JobChanged(job.snapshot.clone()));}
                            if job.snapshot.state == JobState::Running && job.cancel.try_pause() {
                                job.snapshot.state = JobState::Pausing;
                                job.snapshot.bytes_per_second = 0; job.snapshot.eta_seconds = None;
                                bus.publish(Event::JobChanged(job.snapshot.clone()));
                            }
                        }
                    },
                    Some(Command::Resume(id)) => {
                        if let Some(job) = jobs.get_mut(&id) {
                            job.user_held = false;
                            if job.snapshot.state == JobState::Paused {
                                job.snapshot.state = JobState::Waiting;
                                waiting.push_back(id);
                                bus.publish(Event::JobChanged(job.snapshot.clone()));
                            }
                        }
                    },
                    Some(Command::Activate(id)) => {
                        if let Some(job)=jobs.get_mut(&id){if activate_external(job,&mut waiting,&bus).is_err(){reject(&bus,"Supply credentials, confirm the required restart, or wait for the previous result before resuming.");}}
                    },
                    Some(Command::Restart(id)) => {
                        if jobs.get(&id).is_some_and(|j|j.snapshot.readiness!=EntryReadiness::Ready){reject(&bus,"Supply credentials or confirm the required restart before redownloading.");continue;}
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
                    Some(Command::RemoveCompleted) => {
                        // Decide on the actor's current states, so a concurrent
                        // restart cannot turn a toolbar cleanup into cancellation.
                        let completed:Vec<_>=jobs.iter().filter(|(id,job)|job.snapshot.state==JobState::Completed&&!has_retained_partial(job)&&!bus.terminal_pending(**id)).map(|(id,_)|*id).collect();
                        for id in completed{jobs.remove(&id);waiting.retain(|queued|*queued!=id);bus.publish(Event::JobRemoved(id));}
                    },
                    Some(Command::Remove(id)) => {
                        if jobs.get(&id).is_some_and(has_retained_partial) { reject(&bus,"Recovery files must be resolved before removing this entry. Use the required restart action or inspect its recovery paths.");continue; }
                        if jobs.get(&id).is_some_and(|job| job.snapshot.state.is_terminal() && has_retained_partial(job)) && bus.terminal_pending(id) { reject(&bus, "A partial file needs attention. Read its recovery path before removing the entry."); continue; }
                        if let Some(job) = jobs.get_mut(&id).filter(|job| job.snapshot.state.is_terminal()) {
                            if let Some(recovery) = &job.request.recovery {
                                if let Err(error) = crate::output::discard_checkpoint(recovery) {
                                    job.snapshot.error = Some(error);
                                    bus.publish(Event::JobChanged(job.snapshot.clone()));
                                    continue;
                                }
                                job.snapshot.recovery_paths = recovery.store.snapshot().entries.get(&recovery.entry).map(|entry| entry.retained_files.clone()).unwrap_or_default();
                                if !job.snapshot.recovery_paths.is_empty() {
                                    reject(&bus, "The saved partial changed. Its recovery path was preserved; inspect it before removing this entry.");
                                    bus.publish(Event::JobChanged(job.snapshot.clone()));
                                    continue;
                                }
                            }
                        }
                        if let Some(job) = jobs.get_mut(&id) { job.restart = false; job.resume_after_pause = false; if job.snapshot.state.is_terminal() { jobs.remove(&id); bus.publish(Event::JobRemoved(id)); } else { job.remove = true; cancel_job(job, &bus); if job.snapshot.state.is_terminal() { jobs.remove(&id); bus.publish(Event::JobRemoved(id)); } } }
                    },
                    Some(Command::Retry(id)) => {
                        if jobs.get(&id).is_some_and(|j|j.snapshot.readiness!=EntryReadiness::Ready){reject(&bus,"Supply credentials or confirm the required restart before retrying.");continue;}
                        if let Some(job) = jobs.get_mut(&id) {
                            if !job.snapshot.state.can_retry() { reject(&bus, "Only failed or cancelled entries can be retried."); continue; }
                            if has_retained_partial(job) { reject(&bus, "A recovery file needs attention. Inspect its path before retrying."); continue; }
                            if bus.terminal_pending(id) { reject(&bus, "Wait for the previous result to appear before retrying."); continue; }
                            if restart_job(job, &bus) { waiting.push_back(id); }
                        }
                    },
                    Some(_) => reject(&bus, "Unsupported queue command."),
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
                        Ok(TransferOutcome::Cancelled) if job.cancel.preserving() => {
                            if prepare_replies.contains_key(&id) {
                                job.snapshot.state = JobState::Paused;
                                job.snapshot.deferred = true;
                                job.request.deferred = true;
                                job.started = false;
                                job.cancel = TransferControl::new(CancellationToken::new());
                                job.cancel.set_speed_limit(job.snapshot.speed_limit);
                            } else { job.snapshot.state=JobState::Waiting;job.snapshot.deferred=true; }
                        },
                        Ok(TransferOutcome::Cancelled) => {
                            job.snapshot.state=JobState::Cancelled;
                            if let Some(recovery)=&job.request.recovery {if let Err(error)=crate::output::discard_checkpoint(recovery){job.snapshot.state=JobState::Failed;job.snapshot.error=Some(error);}}
                        },
                        Err(error) => { job.snapshot.state = JobState::Failed; if error.kind==ErrorKind::RestartRequired {job.snapshot.readiness=EntryReadiness::RestartRequired;} job.snapshot.error = Some(error); },
                    }
                    if job.snapshot.state==JobState::Failed && job.request.options.confirmation_required && job.snapshot.readiness==EntryReadiness::Ready {job.snapshot.readiness=EntryReadiness::ConfirmationRequired;}
                    job.snapshot.refreshable = job.request.recovery.as_ref().and_then(|r| r.checkpoint()).is_some_and(|c| c.received() > 0 && c.total.is_some_and(|total| c.received() < total));
                    if let Some(reply) = prepare_replies.remove(&id) {
                        let result = refresh_support::checkpoint(job).map(|_| job.request.source_page.clone());
                        let _ = reply.send(result);
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
            result = refresh_tasks.join_next_with_id(), if !refresh_tasks.is_empty() => {
                let (task_id, verification) = match result.expect("nonempty refresh tasks") {
                    Ok((task_id, (_, verification))) => (task_id, verification),
                    Err(error) => (error.id(), Err(JobError::new(ErrorKind::Io, "The refresh verification worker stopped unexpectedly."))),
                };
                let Some(job_id) = refresh_task_jobs.remove(&task_id) else { continue; };
                let Some(pending) = pending_refreshes.remove(&job_id) else { continue; };
                match pending.reply {
                    RefreshReply::Manual(reply) => {
                        let outcome = verification.map_err(|e| e.message).and_then(|verified| {
                            refresh_support::commit_and_queue(&mut jobs, &mut waiting, &bus, job_id, &pending.checkpoint, pending.candidate, verified)
                        });
                        if let Err(error) = &outcome { reject(&bus, error); }
                        let _ = reply.send(outcome);
                    }
                    RefreshReply::Browser { id, lease, reply } => {
                        let outcome = match verification {
                            Ok(verified) => lease.try_accept(|| {
                                refresh_support::commit_and_queue(&mut jobs, &mut waiting, &bus, job_id, &pending.checkpoint, pending.candidate, verified)
                                    .map_err(|error| { reject(&bus, &error); SubmissionError::RecoveryRequired })?;
                                Ok(SubmissionReceipt { id: id.clone(), job_id, created_new: false })
                            }).map_err(|error| match error {
                                LeaseFailure::Expired => SubmissionError::Expired,
                                LeaseFailure::Revoked => SubmissionError::Revoked,
                                LeaseFailure::AlreadyAccepted => SubmissionError::InvalidAction,
                                LeaseFailure::Action(error) => error,
                            }),
                            Err(error) => {
                                reject(&bus, &error.message);
                                Err(SubmissionError::RecoveryRequired)
                            }
                        };
                        if outcome.is_err() { lease.revoke(); }
                        receipts.insert(id, Record { action: SubmissionAction::Refresh(job_id), result: outcome.clone(), cancelled: false, at: Instant::now() });
                        let _ = reply.send(outcome);
                    }
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
                    if job.request.options.confirmation_required && update.paused && job.snapshot.state==JobState::Running {job.snapshot.state=JobState::Paused;}
                    if job.snapshot.state == JobState::Pausing && update.paused {
                        job.snapshot.state = JobState::Paused;
                        if job.resume_after_pause { job.resume_after_pause = false; job.snapshot.state = JobState::Waiting; waiting.push_back(*id); }
                    }
                    job.snapshot.received = update.received; job.snapshot.total = update.total;
                    job.snapshot.resume_capable = update.resume_capable;
                    if job.snapshot.state == JobState::Paused {
                        job.snapshot.refreshable = job.request.recovery.as_ref().and_then(|r| r.checkpoint()).is_some_and(|c| c.received() > 0 && c.total.is_some_and(|total| c.received() < total));
                    }
                    job.snapshot.bytes_per_second = if job.snapshot.state == JobState::Running { job.speed_meter.sample(Instant::now(), update.received) } else { 0 };
                    job.snapshot.eta_seconds = update.total.and_then(|t| t.checked_sub(update.received)).and_then(|remaining| remaining.checked_div(job.snapshot.bytes_per_second));
                    bus.publish(Event::JobChanged(job.snapshot.clone()));
                }
            },
        }
        if !shutdown {
            queues.clock_tick(&mut jobs, &mut waiting, &bus);
            queues.pump(&mut jobs, &mut waiting, &bus);
        }
        queues.reconcile(&mut jobs, &bus);
        queues.publish(&jobs, &bus);
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
            let Some(position) = waiting.iter().position(|id| {
                jobs.get(id).is_some_and(|j| {
                    j.snapshot.readiness == EntryReadiness::Ready && queues.slot_available(j, &jobs)
                })
            }) else {
                break;
            };
            let Some(id) = waiting.remove(position) else {
                break;
            };
            let Some(job) = jobs.get_mut(&id) else {
                continue;
            };
            if job.snapshot.state != JobState::Waiting
                || job.snapshot.readiness != EntryReadiness::Ready
            {
                continue;
            }
            job.snapshot.restored = false;
            job.snapshot.state = JobState::Running;
            if !job.started {
                job.snapshot.last_try = Some(SystemTime::now());
            }
            job.speed_meter.reset(Instant::now(), job.snapshot.received);
            bus.publish(Event::JobChanged(job.snapshot.clone()));
            if job.started {
                job.cancel.try_resume();
                continue;
            }
            job.cancel
                .require_confirmation(job.request.options.confirmation_required);
            job.cancel.attach_recovery(job.request.recovery.clone());
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
    if !job.snapshot.recovery_paths.is_empty() {
        return true;
    }
    job.snapshot.error.as_ref().is_some_and(|error| {
        error.retained_original.is_some()
            || error.retained_partial.as_ref().is_some_and(|path| {
                job.request
                    .recovery
                    .as_ref()
                    .and_then(|recovery| recovery.checkpoint())
                    .is_none_or(|checkpoint| checkpoint.partial != *path)
            })
    })
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
    job.snapshot.refreshable = false;
    job.snapshot.bytes_per_second = 0;
    job.snapshot.eta_seconds = None;
    job.speed_meter.reset(Instant::now(), 0);
    job.snapshot.deferred = false;
    job.request.deferred = false;
    job.request.replacement = replacement;
    job.cancel = TransferControl::new(CancellationToken::new());
    job.cancel.set_speed_limit(job.snapshot.speed_limit);
    job.started = false;
    job.restart = false;
    job.resume_after_pause = false;
    job.remove = false;
    job.user_held = false;
}
fn restart_job(job: &mut Job, bus: &EventBus) -> bool {
    job.restart = false;
    if let Some(recovery) = &job.request.recovery {
        if let Err(error) = crate::output::discard_checkpoint(recovery) {
            job.snapshot.error = Some(error);
            job.snapshot.state = JobState::Failed;
            bus.publish(Event::JobChanged(job.snapshot.clone()));
            return false;
        }
        job.snapshot.recovery_paths = recovery
            .store
            .snapshot()
            .entries
            .get(&recovery.entry)
            .map(|entry| entry.retained_files.clone())
            .unwrap_or_default();
        if !job.snapshot.recovery_paths.is_empty() {
            job.snapshot.state = JobState::Failed;
            job.snapshot.error = Some(JobError::new(ErrorKind::Cleanup,
                "The old partial changed and was preserved. Inspect its recovery path before restarting."));
            bus.publish(Event::JobChanged(job.snapshot.clone()));
            return false;
        }
    }
    match inspect_destination(&job.request.destination) {
        Ok(Some(_)) => {
            job.snapshot.state = JobState::Failed;
            job.snapshot.received = 0;
            job.snapshot.error = Some(JobError::new(
                ErrorKind::DestinationConflict,
                "The destination now exists. Choose a new filename or approve replacement again.",
            ));
            bus.publish(Event::JobChanged(job.snapshot.clone()));
            false
        }
        Ok(None) => {
            reset_job(job, ReplacementPolicy::Deny);
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
fn shutdown_job(job: &mut Job, bus: &EventBus) {
    // Ordinary downloads now have the same held recovery path as queued jobs.
    let retained = job.request.recovery.is_some();
    if retained {
        if job.started && !job.snapshot.state.is_terminal() {
            job.cancel.preserve_on_exit();
        }
        job.snapshot.deferred = true;
        job.request.deferred = true;
    } else {
        cancel_job(job, bus);
    }
}
fn cancel_job(job: &mut Job, bus: &EventBus) {
    match job.snapshot.state {
        JobState::Waiting if !job.started => {
            if let Some(recovery) = &job.request.recovery {
                if let Err(error) = crate::output::discard_checkpoint(recovery) {
                    job.snapshot.state = JobState::Failed;
                    job.snapshot.error = Some(error);
                    bus.publish(Event::JobChanged(job.snapshot.clone()));
                    return;
                }
            }
            job.snapshot.state = JobState::Cancelled;
        }
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
    queues: &QueueRuntime,
    pending_admission: bool,
) -> Result<JobId, SubmissionError> {
    let mut request = request;
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
    request
        .options
        .validate()
        .map_err(|_| SubmissionError::InvalidAction)?;
    // Every transfer needs a durable checkpoint so ordinary failures can
    // retain their owned partial for address refresh. Ephemeral entries are
    // removed after successful completion or explicit cancellation.
    let entry_id = Some(
        queues
            .admit(&mut request, pending_admission)
            .map_err(|_| SubmissionError::InvalidAction)?,
    );
    bus.register_request(id, &request);
    let snapshot = JobSnapshot {
        id,
        entry_id,
        restored: false,
        recovery_paths: vec![],
        queue_id: request
            .deferred
            .then_some(request.queue.unwrap_or(QueueId::MAIN)),
        readiness: EntryReadiness::Ready,
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
        refreshable: false,
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
            user_held: false,
            speed_meter: SpeedMeter::new(Instant::now(), 0),
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
    if job.snapshot.readiness != EntryReadiness::Ready {
        return Err(SubmissionError::RecoveryRequired);
    }
    job.user_held = false;
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
            if has_retained_partial(job)
                && (job
                    .request
                    .recovery
                    .as_ref()
                    .and_then(|r| r.checkpoint())
                    .is_none()
                    || job
                        .snapshot
                        .error
                        .as_ref()
                        .is_some_and(|e| e.retained_original.is_some()))
            {
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
