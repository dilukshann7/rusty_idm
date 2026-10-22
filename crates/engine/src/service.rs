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
#[path = "service_recovery.rs"]
mod recovery_support;
#[path = "service_refresh.rs"]
mod refresh_support;
#[path = "service_telemetry.rs"]
mod telemetry;

#[cfg(test)]
#[path = "service_history_tests.rs"]
mod history_tests;

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
    transfer_meter: telemetry::TransferMeter,
    retry: recovery_support::RetryRuntime,
    request: DownloadRequest,
    retired_history: Option<crate::storage::Recovery>,
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
    let mut history_retry = Instant::now();
    let mut history_error: Option<String> = None;
    let mut tick = tokio::time::interval(Duration::from_millis(100));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        if queues.store.has_pending_history()
            && (shutdown || history_retry.elapsed() >= Duration::from_secs(5))
        {
            history_retry = Instant::now();
            match queues.store.flush_history() {
                Ok(()) => history_error = None,
                Err(error) => {
                    if history_error.as_ref() != Some(&error) {
                        reject(
                            &bus,
                            &format!("Download history has unsaved changes. {error}"),
                        );
                        history_error = Some(error);
                    }
                }
            }
        }
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
            if let Err(error) = queues.store.flush_history() {
                reject(
                    &bus,
                    &format!("Download history still has unsaved changes at shutdown. {error}"),
                );
            }
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
                                            reset_job(job,replacement.clone()).map_err(|error|{reject(&bus,&error.message);SubmissionError::InvalidAction})?;waiting.retain(|j|j!=id);waiting.push_back(*id);publish_job(job, &bus);(*id,false)
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
                                                let replacement=job.request.replacement.clone(); reset_job(job,replacement).map_err(|error|{reject(&bus,&error.message);SubmissionError::InvalidAction})?; waiting.retain(|j|j!=id); waiting.push_back(*id); publish_job(job, &bus);
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
                                publish_job(job, &bus);
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
                        if admit(request,&mut jobs,&mut waiting,&mut next_id,&bus,&queues,false).is_err() { reject(&bus,"Download admission rejected. Check duplicates and save path. If history is full (10,000 entries or 16 MiB), use Delete or Delete Completed and try again."); }
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
                            publish_job(job, &bus);
                        }
                    },
                    Some(Command::Start(id)) => {
                        if closing.load(Ordering::Acquire) { reject(&bus, "The engine is shutting down."); continue; }
                        if let Some(job) = jobs.get_mut(&id) {
                            if job.snapshot.state != JobState::Waiting || (!job.snapshot.deferred && job.snapshot.retry.is_none()) { reject(&bus, "Only a download saved for later or waiting for recovery can be started."); continue; }
                            recovery_support::start_now(job);
                            job.user_held = false;
                            job.snapshot.deferred = false; job.request.deferred = false;
                            waiting.push_back(id); publish_job(job, &bus);
                        }
                    },
                    Some(Command::Cancel(id)) => { if let Some(job) = jobs.get_mut(&id) { job.restart = false; job.resume_after_pause = false; cancel_job(job, &bus); } },
                    Some(Command::Pause(id)) => {
                        if let Some(job) = jobs.get_mut(&id) {
                            job.user_held = true;
                            if let Some(status) = &mut job.snapshot.retry { job.retry.pause(Instant::now(), status); }
                            if job.snapshot.state == JobState::Waiting {waiting.retain(|queued|*queued!=id);job.snapshot.deferred=true;job.request.deferred=true;publish_job(job, &bus);}
                            if job.snapshot.state == JobState::Running && job.cancel.try_pause() {
                                job.snapshot.state = JobState::Pausing;
                                job.snapshot.bytes_per_second = 0; job.snapshot.eta_seconds = None;
                                publish_job(job, &bus);
                            }
                        }
                    },
                    Some(Command::Resume(id)) => {
                        if let Some(job) = jobs.get_mut(&id) {
                            job.user_held = false;
                            if let Some(status) = &mut job.snapshot.retry {
                                job.retry.resume(Instant::now(), SystemTime::now(), status);
                                publish_job(job, &bus);
                                continue;
                            }
                            if job.snapshot.state == JobState::Paused {
                                job.snapshot.state = JobState::Waiting;
                                waiting.push_back(id);
                                publish_job(job, &bus);
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
                            if let Err(error) = prune_missing_recovery(job) { reject(&bus, &error.message); continue; }
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
                                publish_job(job, &bus);
                            }
                        }
                    },
                    Some(Command::CancelAll) => { for job in jobs.values_mut() { job.restart = false; job.resume_after_pause = false; cancel_job(job, &bus); } },
                    Some(Command::RemoveCompleted) => {
                        // Decide on the actor's current states, so a concurrent
                        // restart cannot turn a toolbar cleanup into cancellation.
                        let completed:Vec<_>=jobs.iter().filter(|(id,job)|job.snapshot.state==JobState::Completed&&!has_retained_partial(job)&&!bus.terminal_pending(**id)).map(|(id,_)|*id).collect();
                        let entries: Vec<_> = completed.iter().filter_map(|id|jobs[id].snapshot.entry_id).collect();
                        if let Err(error) = queues.remove_entries(&entries) { reject(&bus, &error); continue; }
                        for id in completed{jobs.remove(&id);waiting.retain(|queued|*queued!=id);bus.publish(Event::JobRemoved(id));}
                    },
                    Some(Command::Remove(id)) => {
                        if let Some(job) = jobs.get_mut(&id) {
                            if let Err(error) = prune_missing_recovery(job) { reject(&bus, &error.message); continue; }
                        }
                        if jobs.get(&id).is_some_and(has_retained_partial) { reject(&bus,"Recovery files must be resolved before removing this entry. Use the required restart action or inspect its recovery paths.");continue; }
                        if jobs.get(&id).is_some_and(|job| job.snapshot.state.is_terminal() && has_retained_partial(job)) && bus.terminal_pending(id) { reject(&bus, "A partial file needs attention. Read its recovery path before removing the entry."); continue; }
                        if let Some(job) = jobs.get_mut(&id).filter(|job| job.snapshot.state.is_terminal()) {
                            if let Some(recovery) = &job.request.recovery {
                                if let Err(error) = crate::output::discard_checkpoint(recovery) {
                                    job.snapshot.error = Some(error);
                                    publish_job(job, &bus);
                                    continue;
                                }
                                job.snapshot.recovery_paths = recovery.store.snapshot().entries.get(&recovery.entry).map(|entry| entry.retained_files.clone()).unwrap_or_default();
                                if !job.snapshot.recovery_paths.is_empty() {
                                    reject(&bus, "The saved partial changed. Its recovery path was preserved; inspect it before removing this entry.");
                                    publish_job(job, &bus);
                                    continue;
                                }
                            }
                        }
                        if let Some(job) = jobs.get_mut(&id) {
                            job.restart = false; job.resume_after_pause = false;
                            if !job.snapshot.state.is_terminal() { job.remove = true; cancel_job(job, &bus); }
                            if job.snapshot.state.is_terminal() {
                                let entries: Vec<_> = job.snapshot.entry_id.into_iter().collect();
                                if let Err(error) = queues.remove_entries(&entries) { job.remove=false; reject(&bus,&error); continue; }
                                jobs.remove(&id); bus.publish(Event::JobRemoved(id));
                            }
                        }
                    },
                    Some(Command::Retry(id)) => {
                        if jobs.get(&id).is_some_and(|j|j.snapshot.readiness!=EntryReadiness::Ready){reject(&bus,"Supply credentials or confirm the required restart before retrying.");continue;}
                        if let Some(job) = jobs.get_mut(&id) {
                            if !job.snapshot.state.can_retry() { reject(&bus, "Only failed or cancelled entries can be retried."); continue; }
                            if let Err(error) = prune_missing_recovery(job) { reject(&bus, &error.message); continue; }
                            if has_unmanaged_recovery(job) { reject(&bus, "A recovery file needs attention. Inspect its path before retrying."); continue; }
                            if bus.terminal_pending(id) { reject(&bus, "Wait for the previous result to appear before retrying."); continue; }
                            if restart_job_inner(job, &bus, true) { waiting.push_back(id); }
                        }
                    },
                    Some(_) => reject(&bus, "Unsupported queue command."),
                }
            },
            result = tasks.join_next_with_id(), if !tasks.is_empty() => {
                let (task_id, result) = match result.expect("nonempty tasks") { Ok((id, result)) => (id, result), Err(error) => (error.id(), Err(JobError::new(ErrorKind::Io, "The transfer worker stopped unexpectedly."))) };
                let Some(id) = task_jobs.remove(&task_id) else { continue; };
                if let Some(job) = jobs.get_mut(&id) {
                    if let Some(p) = progress.lock().unwrap_or_else(|e| e.into_inner()).remove(&id) { job.snapshot.received = p.received; job.snapshot.total = p.total; job.snapshot.resume_capable = p.resume_capable; if let Some(details) = p.details { job.transfer_meter.update(details, Instant::now()); } }
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
                    if job.request.media.is_some() {
                        if let Some(recovery) = &job.request.recovery {
                            job.snapshot.recovery_paths = recovery.store.snapshot().entries.get(&recovery.entry)
                                .map(|entry| entry.retained_files.clone()).unwrap_or_default();
                        }
                    }
                    if job.snapshot.state==JobState::Failed && job.request.options.confirmation_required && job.snapshot.readiness==EntryReadiness::Ready {job.snapshot.readiness=EntryReadiness::ConfirmationRequired;}
                    job.snapshot.refreshable = job.request.media.is_none() && job.request.recovery.as_ref().and_then(|r| r.checkpoint()).is_some_and(|c| c.received() > 0 && c.total.is_some_and(|total| c.received() < total));
                    if let Some(reply) = prepare_replies.remove(&id) {
                        let result = refresh_support::checkpoint(job).map(|_| job.request.source_page.clone());
                        let _ = reply.send(result);
                    }
                    job.transfer_meter.finish(Instant::now(), job.snapshot.state);
                    job.snapshot.bytes_per_second = 0; job.snapshot.eta_seconds = None;
                    if job.restart && !shutdown && !closing.load(Ordering::Acquire) && !has_retained_partial(job) {
                        // Restart is serialized after cleanup, including a late
                        // completion that beat cancellation. No shared partial.
                        if restart_job(job, &bus) { waiting.push_back(id); }
                        continue;
                    }
                    if !shutdown && !closing.load(Ordering::Acquire) {
                        recovery_support::schedule(job, Instant::now(), SystemTime::now());
                    }
                    job.restart = false;
                    publish_job(job, &bus);
                    if job.remove && !has_retained_partial(job) {
                        let entries: Vec<_> = job.snapshot.entry_id.into_iter().collect();
                        match queues.remove_entries(&entries) {
                            Ok(()) => { jobs.remove(&id); bus.publish(Event::JobRemoved(id)); }
                            Err(error) => { job.remove=false; reject(&bus,&error); }
                        }
                    }
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
                    let update = updates.get(id).cloned().unwrap_or(TransferProgress { received: job.snapshot.received, total: job.snapshot.total, resume_capable: job.snapshot.resume_capable, paused: false, details: None });
                    if job.request.options.confirmation_required && update.paused && job.snapshot.state==JobState::Running {job.snapshot.state=JobState::Paused;}
                    if job.snapshot.state == JobState::Pausing && update.paused {
                        job.snapshot.state = JobState::Paused;
                        if job.resume_after_pause { job.resume_after_pause = false; job.snapshot.state = JobState::Waiting; waiting.push_back(*id); }
                    }
                    job.snapshot.received = update.received; job.snapshot.total = update.total;
                    job.snapshot.resume_capable = update.resume_capable;
                    if let Some(details) = update.details { job.transfer_meter.update(details, Instant::now()); }
                    if job.snapshot.state == JobState::Paused {
                        job.snapshot.refreshable = job.request.recovery.as_ref().and_then(|r| r.checkpoint()).is_some_and(|c| c.received() > 0 && c.total.is_some_and(|total| c.received() < total));
                    }
                    job.snapshot.bytes_per_second = if job.snapshot.state == JobState::Running { job.speed_meter.sample(Instant::now(), update.received) } else { 0 };
                    job.snapshot.eta_seconds = update.total.and_then(|t| t.checked_sub(update.received)).and_then(|remaining| remaining.checked_div(job.snapshot.bytes_per_second));
                    publish_job(job, &bus);
                }
            },
        }
        if !shutdown {
            queues.clock_tick(&mut jobs, &mut waiting, &bus);
            recovery_support::pump(
                &mut jobs,
                &mut waiting,
                &queues,
                &bus,
                Instant::now(),
                SystemTime::now(),
            );
            queues.pump(&mut jobs, &mut waiting, &bus);
        }
        queues.reconcile(&mut jobs, &bus);
        queues.publish(&jobs, &bus);
        // Queued cancellations cannot admit work; cleanup must release the task slot.
        while !shutdown
            && !queues.store.has_pending_history()
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
                    j.snapshot.readiness == EntryReadiness::Ready
                        && queues.slot_available(j, &jobs)
                        && recovery_support::ready(j, &queues, Instant::now())
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
            if job.snapshot.retry.take().is_some() {
                job.retry.admitted();
                job.snapshot.error = None;
            }
            if !job.started {
                job.snapshot.last_try = Some(SystemTime::now());
            }
            job.speed_meter.reset(Instant::now(), job.snapshot.received);
            if !job.started {
                job.transfer_meter = Default::default();
                job.snapshot.transfer_details = None;
            }
            publish_job(job, &bus);
            if queues.store.has_pending_history() {
                job.snapshot.state = JobState::Waiting;
                job.snapshot.deferred = true;
                job.request.deferred = true;
                job.user_held = true;
                publish_job(job, &bus);
                continue;
            }
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
                let mut pending = updates.lock().unwrap_or_else(|e| e.into_inner());
                if let Some(previous) = pending.get_mut(&id) {
                    previous.coalesce(p);
                } else {
                    pending.insert(id, p);
                }
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
fn publish_job(job: &mut Job, bus: &EventBus) {
    job.snapshot.transfer_details = job
        .transfer_meter
        .snapshot(Instant::now(), job.snapshot.state);
    if let Some(details) = &job.snapshot.transfer_details {
        job.snapshot.bytes_per_second = details
            .segments
            .iter()
            .fold(0u64, |sum, row| sum.saturating_add(row.bytes_per_second));
        job.snapshot.eta_seconds = if job.snapshot.state == JobState::Running {
            job.snapshot
                .total
                .and_then(|total| total.checked_sub(job.snapshot.received))
                .and_then(|remaining| remaining.checked_div(job.snapshot.bytes_per_second))
        } else {
            None
        };
    }
    if !job.snapshot.restored {
        if let Some(recovery) = &job.request.recovery {
            if let Some(saved) = recovery.store.history(recovery.entry) {
                let terminal = matches!(job.snapshot.state, JobState::Completed | JobState::Failed);
                if terminal && job.snapshot.finished_at.is_none() {
                    let outcome = if job.snapshot.state == JobState::Completed {
                        crate::history::HistoryOutcome::Completed
                    } else {
                        crate::history::HistoryOutcome::Failed
                    };
                    job.snapshot.finished_at = (saved.outcome == Some(outcome))
                        .then_some(saved.finished_at)
                        .flatten()
                        .or_else(|| Some(SystemTime::now()));
                } else if !terminal {
                    job.snapshot.finished_at = None;
                }
                let history = crate::history::HistoryMetadata::from_snapshot(&job.snapshot);
                // Running byte counters belong to durable checkpoints. History
                // writes only lifecycle changes, not each progress notification.
                if (terminal
                    || saved.last_try != history.last_try
                    || (saved.outcome.is_some()
                        && job.snapshot.state == JobState::Waiting
                        && !job.started))
                    && saved != history
                {
                    if let Err(error) = recovery.save_history(history) {
                        reject(
                            bus,
                            &format!("Download history has unsaved changes. {}", error.message),
                        );
                    }
                }
            }
        }
    }
    bus.publish(Event::JobChanged(job.snapshot.clone()));
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
fn has_unmanaged_recovery(job: &Job) -> bool {
    let stage = job
        .request
        .media
        .as_ref()
        .and_then(|_| crate::media::stage_directory(&job.request).ok());
    if job
        .snapshot
        .recovery_paths
        .iter()
        .any(|path| Some(path) != stage.as_ref())
    {
        return true;
    }
    job.snapshot.error.as_ref().is_some_and(|error| {
        error.retained_original.is_some()
            || error.retained_partial.as_ref().is_some_and(|path| {
                Some(path) != stage.as_ref()
                    && job
                        .request
                        .recovery
                        .as_ref()
                        .and_then(|recovery| recovery.checkpoint())
                        .is_none_or(|checkpoint| checkpoint.partial != *path)
            })
    })
}
fn prune_missing_recovery(job: &mut Job) -> Result<(), JobError> {
    let missing = |path: &std::path::Path| {
        std::fs::symlink_metadata(path)
            .is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound)
    };
    if let Some(recovery) = &job.request.recovery {
        recovery
            .store
            .update(|state| {
                if let Some(entry) = state.entries.get_mut(&recovery.entry) {
                    entry.retained_files.retain(|path| !missing(path));
                }
                Ok(())
            })
            .map_err(crate::storage::storage_error)?;
    }
    job.snapshot.recovery_paths.retain(|path| !missing(path));
    if let Some(error) = &mut job.snapshot.error {
        if error
            .retained_partial
            .as_ref()
            .is_some_and(|path| missing(path))
        {
            error.retained_partial = None;
        }
        if error
            .retained_original
            .as_ref()
            .is_some_and(|path| missing(path))
        {
            error.retained_original = None;
        }
    }
    Ok(())
}
fn same_destination(a: &std::path::Path, b: &std::path::Path) -> bool {
    a.to_string_lossy().replace('/', "\\").to_lowercase()
        == b.to_string_lossy().replace('/', "\\").to_lowercase()
}
fn restore_retired_history(job: &mut Job) -> Result<(), JobError> {
    if let Some(recovery) = job.retired_history.as_ref() {
        recovery
            .store
            .update(|state| {
                if !state.entries.contains_key(&recovery.entry) {
                    let mut entry = queue_support::stored(recovery.entry, &job.request);
                    entry.history.created_at = job.snapshot.created_at;
                    state.entries.insert(recovery.entry, entry);
                }
                Ok(())
            })
            .map_err(crate::storage::storage_error)?;
        job.snapshot.entry_id = Some(recovery.entry);
        job.request.recovery = job.retired_history.take();
    }
    Ok(())
}
fn reset_job(job: &mut Job, replacement: ReplacementPolicy) -> Result<(), JobError> {
    restore_retired_history(job)?;
    recovery_support::clear(job);
    job.snapshot.restored = false;
    job.snapshot.state = JobState::Waiting;
    job.snapshot.received = 0;
    job.snapshot.total = None;
    job.snapshot.error = None;
    job.snapshot.last_try = None;
    job.snapshot.finished_at = None;
    job.snapshot.resume_capable = false;
    job.snapshot.refreshable = false;
    job.snapshot.bytes_per_second = 0;
    job.snapshot.eta_seconds = None;
    job.speed_meter.reset(Instant::now(), 0);
    job.transfer_meter = Default::default();
    job.snapshot.transfer_details = None;
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
    Ok(())
}
fn restart_job(job: &mut Job, bus: &EventBus) -> bool {
    restart_job_inner(job, bus, false)
}
fn restart_job_inner(job: &mut Job, bus: &EventBus, keep_media_stage: bool) -> bool {
    job.restart = false;
    if let Err(error) = restore_retired_history(job) {
        reject(bus, &error.message);
        return false;
    }
    if let Some(recovery) = &job.request.recovery {
        if let Err(error) = crate::output::discard_checkpoint(recovery) {
            job.snapshot.error = Some(error);
            job.snapshot.state = JobState::Failed;
            publish_job(job, &bus);
            return false;
        }
        job.snapshot.recovery_paths = recovery
            .store
            .snapshot()
            .entries
            .get(&recovery.entry)
            .map(|entry| entry.retained_files.clone())
            .unwrap_or_default();
        if if keep_media_stage {
            has_unmanaged_recovery(job)
        } else {
            !job.snapshot.recovery_paths.is_empty()
        } {
            job.snapshot.state = JobState::Failed;
            job.snapshot.error = Some(JobError::new(ErrorKind::Cleanup,
                "The old partial changed and was preserved. Inspect its recovery path before restarting."));
            publish_job(job, &bus);
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
            publish_job(job, &bus);
            false
        }
        Ok(None) => {
            if let Err(error) = reset_job(job, ReplacementPolicy::Deny) {
                reject(bus, &error.message);
                return false;
            }
            publish_job(job, &bus);
            true
        }
        Err(error) => {
            job.snapshot.state = JobState::Failed;
            job.snapshot.error = Some(error);
            publish_job(job, &bus);
            false
        }
    }
}
fn shutdown_job(job: &mut Job, bus: &EventBus) {
    recovery_support::clear(job);
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
    let recovering = job.snapshot.retry.is_some();
    recovery_support::clear(job);
    if recovering {
        job.snapshot.error = None;
    }
    match job.snapshot.state {
        JobState::Waiting if !job.started => {
            if let Some(recovery) = &job.request.recovery {
                if let Err(error) = crate::output::discard_checkpoint(recovery) {
                    job.snapshot.state = JobState::Failed;
                    job.snapshot.error = Some(error);
                    publish_job(job, &bus);
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
    publish_job(job, &bus);
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
    if jobs.len() + bus.removed_pending() >= crate::history::MAX_HISTORY_ENTRIES {
        reject(bus, crate::history::HISTORY_FULL);
        return Err(SubmissionError::InvalidAction);
    }
    if queues.store.has_pending_history() {
        reject(bus, "Download history has unsaved changes. Retry after storage becomes available, or delete history entries to free capacity.");
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
    if request
        .media
        .as_ref()
        .is_some_and(|selection| selection.validate().is_err())
    {
        return Err(SubmissionError::InvalidAction);
    }
    // Every transfer needs a durable checkpoint so ordinary failures can
    // retain their owned partial for address refresh. Completed and failed
    // records remain history; ordinary explicit cancellations are ephemeral.
    let entry_id = Some(
        queues
            .admit(&mut request, pending_admission)
            .map_err(|error| {
                reject(bus, &error);
                SubmissionError::InvalidAction
            })?,
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
        transfer_details: None,
        eta_seconds: None,
        created_at: request.recovery.as_ref().and_then(|r| {
            r.store
                .snapshot()
                .entries
                .get(&r.entry)
                .and_then(|e| e.history.created_at)
        }),
        finished_at: None,
        last_try: None,
        retry: None,
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
            transfer_meter: Default::default(),
            retry: Default::default(),
            request,
            retired_history: None,
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
    if job.snapshot.retry.is_some() {
        recovery_support::start_now(job);
        job.snapshot.deferred = false;
        job.request.deferred = false;
        waiting.retain(|id| *id != job.snapshot.id);
        waiting.push_back(job.snapshot.id);
        publish_job(job, bus);
        return Ok(());
    }
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
            reset_job(job, ReplacementPolicy::Deny).map_err(|error| {
                reject(bus, &error.message);
                SubmissionError::InvalidAction
            })?;
            waiting.push_back(job.snapshot.id);
        }
        JobState::Cancelling => return Err(SubmissionError::Busy),
        _ => {}
    }
    publish_job(job, &bus);
    Ok(())
}

#[cfg(test)]
mod media_recovery_tests {
    use super::*;

    #[tokio::test]
    async fn restored_protected_entry_cannot_skip_capture_by_confirming_restart() {
        let directory = tempfile::tempdir().unwrap();
        let url = "https://example.com/file";
        let mut request = validate_request(
            url,
            directory.path().join("file.bin"),
            ReplacementPolicy::Deny,
        )
        .unwrap()
        .with_browser_context(vec![rusty_browser_bridge::protocol::RequestHop {
            url: url.into(),
            headers: vec![rusty_browser_bridge::protocol::RequestHeader {
                name: "Cookie".into(),
                value: "session=private".into(),
            }],
        }])
        .unwrap();
        request.options.confirmation_required = true;
        request = request.with_authorization("login".into(), "private-password".into());
        let mut queues = QueueRuntime::new(false);
        let entry = queues.admit(&mut request, false).unwrap();
        let bus = EventBus::new(Arc::new(|| {}));
        let mut jobs = BTreeMap::new();
        queues.restore(&mut jobs, &mut 1, &bus);
        let id = *jobs.keys().next().unwrap();
        assert!(jobs[&id].request.request_context.is_empty());
        assert_eq!(
            jobs[&id].snapshot.readiness,
            EntryReadiness::RestartRequired
        );
        let mut waiting = VecDeque::new();
        queues
            .command(Command::ConfirmRestart(id), &mut jobs, &mut waiting, &bus)
            .await;
        assert_eq!(
            jobs[&id].snapshot.readiness,
            EntryReadiness::RestartRequired
        );
        assert!(waiting.is_empty());
        assert!(queues.store.snapshot().entries[&entry].refresh_needs_capture);
    }

    #[test]
    #[cfg(windows)]
    fn failed_and_cancelled_media_can_retry_owned_stage_but_still_guard_destination() {
        for state in [JobState::Failed, JobState::Cancelled] {
            let directory = tempfile::tempdir().unwrap();
            let destination = directory.path().join("video.mp4");
            let mut request = validate_request(
                "https://example.com/master.m3u8",
                destination.clone(),
                ReplacementPolicy::Deny,
            )
            .unwrap();
            request.media = Some(crate::media::MediaSelection {
                identity: "Generic:public".into(),
                format_id: "720".into(),
                container: "mp4".into(),
                quality: "720p".into(),
                audio: "aac".into(),
                kind: rusty_browser_bridge::protocol::MediaKind::Hls,
            });
            let queues = QueueRuntime::new(false);
            let bus = EventBus::new(Arc::new(|| {}));
            let mut jobs = BTreeMap::new();
            let id = admit(
                request,
                &mut jobs,
                &mut VecDeque::new(),
                &mut 1,
                &bus,
                &queues,
                false,
            )
            .unwrap();
            let job = jobs.get_mut(&id).unwrap();
            let stage = crate::media::stage_directory(&job.request).unwrap();
            let recovery = job.request.recovery.as_ref().unwrap().clone();
            recovery
                .store
                .update(|saved| {
                    saved
                        .entries
                        .get_mut(&recovery.entry)
                        .unwrap()
                        .retained_files
                        .push(stage.clone());
                    Ok(())
                })
                .unwrap();
            job.snapshot.state = state;
            job.snapshot.recovery_paths.push(stage.clone());
            job.snapshot.error =
                Some(JobError::new(ErrorKind::Io, "Interrupted media").retained(stage.clone()));
            assert!(has_retained_partial(job));
            assert!(!has_unmanaged_recovery(job));
            std::fs::write(&destination, b"existing destination").unwrap();
            assert!(!restart_job_inner(job, &bus, true));
            assert_eq!(
                std::fs::read(&destination).unwrap(),
                b"existing destination"
            );
            std::fs::remove_file(&destination).unwrap();
            assert!(restart_job_inner(job, &bus, true));
            assert!(recovery.store.snapshot().entries[&recovery.entry]
                .retained_files
                .contains(&stage));
        }
    }

    #[test]
    fn removed_recovery_paths_can_be_acknowledged_without_deleting_existing_files() {
        let directory = tempfile::tempdir().unwrap();
        let retained = directory.path().join("retained.bin");
        std::fs::write(&retained, b"keep").unwrap();
        let request = validate_request(
            "https://example.com/file",
            directory.path().join("output.bin"),
            ReplacementPolicy::Deny,
        )
        .unwrap();
        let queues = QueueRuntime::new(false);
        let bus = EventBus::new(Arc::new(|| {}));
        let mut jobs = BTreeMap::new();
        let id = admit(
            request,
            &mut jobs,
            &mut VecDeque::new(),
            &mut 1,
            &bus,
            &queues,
            false,
        )
        .unwrap();
        let job = jobs.get_mut(&id).unwrap();
        let recovery = job.request.recovery.as_ref().unwrap().clone();
        recovery
            .store
            .update(|saved| {
                saved
                    .entries
                    .get_mut(&recovery.entry)
                    .unwrap()
                    .retained_files
                    .push(retained.clone());
                Ok(())
            })
            .unwrap();
        job.snapshot.recovery_paths.push(retained.clone());
        job.snapshot.error =
            Some(JobError::new(ErrorKind::Cleanup, "Retained").retained(retained.clone()));
        prune_missing_recovery(job).unwrap();
        assert!(has_retained_partial(job));
        assert_eq!(std::fs::read(&retained).unwrap(), b"keep");
        std::fs::remove_file(&retained).unwrap();
        prune_missing_recovery(job).unwrap();
        assert!(!has_retained_partial(job));
        assert!(recovery.store.snapshot().entries[&recovery.entry]
            .retained_files
            .is_empty());
    }
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
