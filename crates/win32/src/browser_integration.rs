use rusty_browser_bridge::{
    lease::{CaptureLease, LeaseState},
    protocol::*,
    windows::*,
};
use rusty_engine::{refresh::RefreshCandidate, submissions::*, CommandError, EngineClient, JobId};
use std::{
    collections::{HashMap, VecDeque},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, SyncSender},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};
use tokio::sync::oneshot;
struct Entry {
    capture: CaptureRequest,
    lease: CaptureLease,
    original: Option<ReplyStatus>,
    current: Option<ReplyStatus>,
    waiters: Vec<SyncSender<Reply>>,
    at: Instant,
}
struct MediaEntry {
    request: MediaSubmitRequest,
    lease: CaptureLease,
    result: Option<ReplyStatus>,
    waiters: Vec<SyncSender<Reply>>,
    at: Instant,
}
pub struct IncomingMedia {
    pub id: RequestId,
    pub preview: rusty_engine::media::MediaPreview,
    pub selection: rusty_engine::media::MediaSelection,
    pub lease: CaptureLease,
}
#[derive(Default)]
struct State {
    policy: rusty_browser_bridge::policy::CapturePolicy,
    refresh_armed: bool,
    entries: HashMap<RequestId, Entry>,
    media_previews: HashMap<RequestId, (rusty_engine::media::MediaPreview, BrowserKind, Instant)>,
    media_inspections: usize,
    media_entries: HashMap<RequestId, MediaEntry>,
    media_inbox: VecDeque<IncomingMedia>,
    inbox: VecDeque<IncomingCapture>,
    activate: bool,
    presentations: VecDeque<rusty_engine::JobId>,
    closing: bool,
}
impl State {
    fn media_capacity(&self) -> bool {
        self.media_entries.len() < MAX_RECEIPTS
            && self
                .media_entries
                .values()
                .filter(|entry| entry.result.is_none())
                .count()
                < MAX_PENDING
    }
    fn finish(&mut self, id: &RequestId, status: ReplyStatus) {
        if let Some(entry) = self.entries.get_mut(id) {
            if entry.original.is_none() {
                if let ReplyStatus::Accepted { job_id, .. } = &status {
                    let job_id = rusty_engine::JobId(*job_id);
                    if self.presentations.len() < MAX_RECEIPTS
                        && !self.presentations.contains(&job_id)
                    {
                        self.presentations.push_back(job_id);
                    }
                }
                entry.original = Some(status.clone());
                for waiter in entry.waiters.drain(..) {
                    let _ = waiter.try_send(Reply::new(id.clone(), status.clone()));
                }
            }
            entry.current = Some(status);
        }
        if let Some(entry) = self.media_entries.get_mut(id) {
            if entry.result.is_none() {
                if let ReplyStatus::Accepted { job_id, .. } = &status {
                    let job_id = JobId(*job_id);
                    if self.presentations.len() < MAX_RECEIPTS
                        && !self.presentations.contains(&job_id)
                    {
                        self.presentations.push_back(job_id);
                    }
                }
                for waiter in entry.waiters.drain(..) {
                    let _ = waiter.try_send(Reply::media(id.clone(), status.clone()));
                }
                entry.result = Some(status);
            }
        }
    }
    fn close(&mut self) {
        self.closing = true;
        self.inbox.clear();
        self.media_inbox.clear();
        let ids: Vec<_> = self
            .entries
            .iter()
            .filter_map(|(id, e)| (e.lease.revoke() != LeaseState::Accepted).then_some(id.clone()))
            .collect();
        for id in ids {
            self.finish(
                &id,
                ReplyStatus::Declined {
                    code: DeclineCode::AppClosing,
                },
            );
        }
        let media: Vec<_> = self
            .media_entries
            .iter()
            .filter_map(|(id, entry)| {
                (entry.result.is_none() && entry.lease.revoke() != LeaseState::Accepted)
                    .then_some(id.clone())
            })
            .collect();
        for id in media {
            self.finish(
                &id,
                ReplyStatus::Declined {
                    code: DeclineCode::AppClosing,
                },
            );
        }
    }
}
enum Work {
    Apply(RequestId, SubmissionAction, CaptureLease),
    Refresh(RequestId, JobId, RefreshCandidate, CaptureLease),
    Status(RequestId, SyncSender<Reply>),
    Abort(RequestId, SyncSender<Reply>),
}
enum Waiting {
    Apply(
        RequestId,
        oneshot::Receiver<Result<SubmissionReceipt, SubmissionError>>,
    ),
    Status(
        RequestId,
        SyncSender<Reply>,
        oneshot::Receiver<SubmissionStatus>,
    ),
    Abort(
        RequestId,
        SyncSender<Reply>,
        oneshot::Receiver<CancelOwnedResult>,
    ),
}
struct Router {
    state: Arc<Mutex<State>>,
    work: SyncSender<Work>,
    wake: Arc<dyn Fn() + Send + Sync>,
}
fn target_id() -> RequestId {
    RequestId::parse("00000000-0000-4000-8000-000000000000").expect("fixed identifier")
}
impl Router {
    fn media_inspect(&self, id: RequestId, request: MediaInspectRequest, reply: SyncSender<Reply>) {
        {
            let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
            if state.closing
                || !state.policy.panels.enabled
                || !state.policy.browsers.contains(&request.browser)
                || state.policy.excluded(&request.url)
            {
                let _ = reply.try_send(Reply::media(
                    id,
                    ReplyStatus::Declined {
                        code: DeclineCode::Unsupported,
                    },
                ));
                return;
            }
            if state.media_inspections >= 4 {
                let _ = reply.try_send(Reply::media(
                    id,
                    ReplyStatus::Declined {
                        code: DeclineCode::Busy,
                    },
                ));
                return;
            }
            state.media_inspections += 1;
        }
        let _ = reply.try_send(Reply::media(id.clone(), ReplyStatus::Pending));
        let state = self.state.clone();
        thread::spawn(move || {
            let inspected = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .ok()
                .and_then(|runtime| {
                    runtime
                        .block_on(rusty_engine::media::inspect(
                            &request.url,
                            request.kind.clone(),
                            tokio_util::sync::CancellationToken::new(),
                        ))
                        .ok()
                });
            let result = if let Some(preview) = inspected {
                let expires = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64
                    + 120_000;
                let status = ReplyStatus::MediaPreview {
                    preview_id: id.clone(),
                    expires_at_ms: expires,
                    title: preview.title.clone(),
                    variants: preview.variants.clone(),
                };
                let mut locked = state.lock().unwrap_or_else(|p| p.into_inner());
                locked.media_inspections = locked.media_inspections.saturating_sub(1);
                if locked.closing {
                    ReplyStatus::Declined {
                        code: DeclineCode::AppClosing,
                    }
                } else {
                    locked
                        .media_previews
                        .retain(|_, (_, _, at)| at.elapsed() < Duration::from_secs(120));
                    if locked.media_previews.len() >= 64 {
                        ReplyStatus::Declined {
                            code: DeclineCode::Busy,
                        }
                    } else {
                        locked
                            .media_previews
                            .insert(id.clone(), (preview, request.browser, Instant::now()));
                        status
                    }
                }
            } else {
                let mut locked = state.lock().unwrap_or_else(|p| p.into_inner());
                locked.media_inspections = locked.media_inspections.saturating_sub(1);
                ReplyStatus::Declined {
                    code: DeclineCode::Unsupported,
                }
            };
            let _ = reply.try_send(Reply::media(id, result));
        });
    }

    fn media_submit(&self, id: RequestId, request: MediaSubmitRequest, reply: SyncSender<Reply>) {
        let (preview, selected, lease) = {
            let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
            state
                .media_entries
                .retain(|_, entry| entry.result.is_none() || entry.at.elapsed() < RETENTION);
            if let Some(entry) = state.media_entries.get_mut(&id) {
                let result = if entry.request != request {
                    ReplyStatus::Declined {
                        code: DeclineCode::InvalidRequest,
                    }
                } else if let Some(status) = &entry.result {
                    status.clone()
                } else if entry.waiters.len() < 4 {
                    entry.waiters.push(reply.clone());
                    ReplyStatus::Pending
                } else {
                    ReplyStatus::Declined {
                        code: DeclineCode::Busy,
                    }
                };
                let _ = reply.try_send(Reply::media(id, result));
                return;
            }
            let Some((preview, browser, at)) =
                state.media_previews.get(&request.preview_id).cloned()
            else {
                let _ = reply.try_send(Reply::media(
                    id,
                    ReplyStatus::Declined {
                        code: DeclineCode::Expired,
                    },
                ));
                return;
            };
            if state.closing
                || !state.policy.panels.enabled
                || !state.policy.browsers.contains(&request.browser)
                || browser != request.browser
                || at.elapsed() >= Duration::from_secs(120)
                || state.policy.excluded(&preview.url)
                || !state.media_capacity()
            {
                let _ = reply.try_send(Reply::media(
                    id,
                    ReplyStatus::Declined {
                        code: DeclineCode::BrowserFallback,
                    },
                ));
                return;
            }
            let Some(selected) = preview
                .variants
                .iter()
                .find(|variant| variant.id == request.variant_id)
                .cloned()
            else {
                let _ = reply.try_send(Reply::media(
                    id,
                    ReplyStatus::Declined {
                        code: DeclineCode::InvalidRequest,
                    },
                ));
                return;
            };
            let lease = CaptureLease::new(Instant::now() + Duration::from_secs(300));
            state.media_entries.insert(
                id.clone(),
                MediaEntry {
                    request,
                    lease: lease.clone(),
                    result: None,
                    waiters: vec![reply.clone()],
                    at: Instant::now(),
                },
            );
            let _ = reply.try_send(Reply::media(id.clone(), ReplyStatus::Pending));
            (preview, selected, lease)
        };
        let state = self.state.clone();
        let wake = self.wake.clone();
        thread::spawn(move || {
            let current = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .ok()
                .and_then(|runtime| {
                    runtime
                        .block_on(rusty_engine::media::inspect(
                            &preview.url,
                            preview.kind.clone(),
                            tokio_util::sync::CancellationToken::new(),
                        ))
                        .ok()
                });
            let mut locked = state.lock().unwrap_or_else(|p| p.into_inner());
            let valid = current.as_ref().is_some_and(|candidate| {
                candidate.identity == preview.identity
                    && candidate.kind == preview.kind
                    && candidate.variants.iter().any(|variant| {
                        variant.id == selected.id
                            && variant.container == selected.container
                            && variant.quality == selected.quality
                            && variant.audio == selected.audio
                    })
            });
            if !valid || lease.state() != LeaseState::Pending || locked.closing {
                locked.finish(
                    &id,
                    ReplyStatus::Declined {
                        code: DeclineCode::Unsupported,
                    },
                );
                return;
            }
            locked.media_inbox.push_back(IncomingMedia {
                id: id.clone(),
                preview: current.expect("validated"),
                selection: rusty_engine::media::MediaSelection {
                    identity: preview.identity,
                    format_id: selected.id,
                    container: selected.container,
                    quality: selected.quality,
                    audio: selected.audio,
                    kind: preview.kind,
                },
                lease,
            });
            wake();
        });
    }
}
impl RequestRouter for Router {
    fn dispatch(&self, operation: Operation, reply: SyncSender<Reply>) -> Result<(), BridgeError> {
        let operation = match operation {
            Operation::MediaInspect { id, request } => {
                self.media_inspect(id, request, reply);
                return Ok(());
            }
            Operation::MediaSubmit { id, request } => {
                self.media_submit(id, request, reply);
                return Ok(());
            }
            other => other,
        };
        let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        state
            .entries
            .retain(|_, e| e.original.is_none() || e.at.elapsed() < RETENTION);
        match operation {
            Operation::MediaInspect { .. } | Operation::MediaSubmit { .. } => {
                unreachable!("handled before lock")
            }
            Operation::Policy(browser) => {
                let _ = reply.try_send(Reply::new(
                    target_id(),
                    ReplyStatus::Policy {
                        browser,
                        policy: state.policy.clone(),
                        refresh_armed: state.refresh_armed,
                    },
                ));
            }
            Operation::Ping => {
                let _ = reply.try_send(Reply::new(
                    target_id(),
                    ReplyStatus::Ok {
                        host_version: env!("CARGO_PKG_VERSION").into(),
                        gui_running: !state.closing,
                    },
                ));
            }
            Operation::Activate => {
                state.activate = true;
                (self.wake)();
                let _ = reply.try_send(Reply::new(
                    target_id(),
                    ReplyStatus::Ok {
                        host_version: env!("CARGO_PKG_VERSION").into(),
                        gui_running: true,
                    },
                ));
            }
            Operation::Capture(capture) => {
                let id = capture.id.clone();
                if let Some(entry) = state.entries.get_mut(&id) {
                    let status = if entry.capture != capture {
                        ReplyStatus::Declined {
                            code: DeclineCode::InvalidRequest,
                        }
                    } else if let Some(result) = &entry.original {
                        result.clone()
                    } else if entry.waiters.len() < 4 {
                        entry.waiters.push(reply.clone());
                        ReplyStatus::Pending
                    } else {
                        ReplyStatus::Declined {
                            code: DeclineCode::Busy,
                        }
                    };
                    let _ = reply.try_send(Reply::new(id, status));
                    return Ok(());
                }
                if !capture.request_context.is_empty()
                    && !state.refresh_armed
                    && !capture.has_credentials()
                {
                    let _ = reply.try_send(Reply::new(
                        id,
                        ReplyStatus::Declined {
                            code: DeclineCode::BrowserFallback,
                        },
                    ));
                    return Ok(());
                }
                let allowed = capture.source != CaptureSource::Automatic
                    || (state.policy.allows(
                        capture.browser,
                        &capture.url,
                        capture.filename_hint.as_deref(),
                        capture.forced
                            && capture.policy_revision == state.policy.revision
                            && state.policy.keys.force != 0,
                    ) && capture
                        .original_url
                        .as_ref()
                        .is_none_or(|url| !state.policy.excluded(url)));
                let code = if !allowed {
                    Some(DeclineCode::BrowserFallback)
                } else if state.closing {
                    Some(DeclineCode::AppClosing)
                } else if state
                    .entries
                    .values()
                    .filter(|e| e.original.is_none())
                    .count()
                    >= MAX_PENDING
                    || state.entries.len() >= MAX_RECEIPTS
                    || state.inbox.len() >= MAX_PENDING
                {
                    Some(DeclineCode::Busy)
                } else {
                    None
                };
                if let Some(code) = code {
                    let _ = reply.try_send(Reply::new(id, ReplyStatus::Declined { code }));
                    return Ok(());
                }
                let lease = CaptureLease::new(capture.deadline());
                if lease.state() != LeaseState::Pending {
                    let _ = reply.try_send(Reply::new(
                        id,
                        ReplyStatus::Declined {
                            code: DeclineCode::Expired,
                        },
                    ));
                    return Ok(());
                }
                state.entries.insert(
                    id.clone(),
                    Entry {
                        capture: capture.clone(),
                        lease: lease.clone(),
                        original: None,
                        current: None,
                        waiters: vec![reply.clone()],
                        at: Instant::now(),
                    },
                );
                state.inbox.push_back(IncomingCapture {
                    capture,
                    lease,
                    reply: reply.clone(),
                });
                let _ = reply.try_send(Reply::new(id, ReplyStatus::Pending));
                (self.wake)();
            }
            Operation::Status { capture_id } => {
                if self
                    .work
                    .try_send(Work::Status(capture_id.clone(), reply.clone()))
                    .is_err()
                {
                    let _ = reply.try_send(Reply::new(
                        capture_id,
                        ReplyStatus::Declined {
                            code: DeclineCode::Busy,
                        },
                    ));
                }
            }
            Operation::Abort { capture_id } => {
                if let Some(entry) = state.entries.get(&capture_id) {
                    if entry.lease.revoke() != LeaseState::Accepted {
                        state.finish(
                            &capture_id,
                            ReplyStatus::Declined {
                                code: DeclineCode::BrowserFallback,
                            },
                        );
                        let _ = reply.try_send(Reply::new(capture_id, ReplyStatus::Aborted));
                        return Ok(());
                    }
                }
                if self
                    .work
                    .try_send(Work::Abort(capture_id.clone(), reply.clone()))
                    .is_err()
                {
                    let _ = reply.try_send(Reply::new(
                        capture_id,
                        ReplyStatus::Declined {
                            code: DeclineCode::Busy,
                        },
                    ));
                }
            }
        }
        Ok(())
    }
}
pub struct BrowserController {
    router: Arc<Router>,
    stop: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}
impl BrowserController {
    pub fn new(engine: EngineClient, wake: Arc<dyn Fn() + Send + Sync>) -> Self {
        let state = Arc::new(Mutex::new(State::default()));
        let (work, rx) = mpsc::sync_channel(64);
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        let shared = state.clone();
        let notify = wake.clone();
        let thread = thread::spawn(move || {
            let mut waiting = Vec::new();
            while !flag.load(Ordering::Acquire) {
                if waiting.len() < 128 {
                    if let Ok(work) = rx.recv_timeout(Duration::from_millis(10)) {
                        match work {
                            Work::Refresh(id, job_id, candidate, lease) => {
                                let (reply, result) = oneshot::channel();
                                if let Err(error) = engine.try_external(ExternalCommand::Refresh {
                                    id: id.clone(),
                                    job_id,
                                    candidate,
                                    lease: lease.clone(),
                                    reply,
                                }) {
                                    lease.revoke();
                                    shared.lock().unwrap_or_else(|p| p.into_inner()).finish(
                                        &id,
                                        ReplyStatus::Declined {
                                            code: command_code(error),
                                        },
                                    );
                                } else {
                                    waiting.push(Waiting::Apply(id, result));
                                }
                            }
                            Work::Apply(id, action, lease) => {
                                let (reply, result) = oneshot::channel();
                                if let Err(e) = engine.try_external(ExternalCommand::Apply {
                                    id: id.clone(),
                                    action,
                                    lease: lease.clone(),
                                    reply,
                                }) {
                                    lease.revoke();
                                    shared.lock().unwrap_or_else(|p| p.into_inner()).finish(
                                        &id,
                                        ReplyStatus::Declined {
                                            code: command_code(e),
                                        },
                                    );
                                } else {
                                    waiting.push(Waiting::Apply(id, result));
                                }
                            }
                            Work::Status(id, tx) => {
                                let (reply, result) = oneshot::channel();
                                if engine
                                    .try_external(ExternalCommand::Status {
                                        id: id.clone(),
                                        reply,
                                    })
                                    .is_err()
                                {
                                    let _ = tx.try_send(Reply::new(id, ReplyStatus::Unknown));
                                } else {
                                    waiting.push(Waiting::Status(id, tx, result));
                                }
                            }
                            Work::Abort(id, tx) => {
                                let (reply, result) = oneshot::channel();
                                if engine
                                    .try_external(ExternalCommand::CancelOwned {
                                        id: id.clone(),
                                        reply,
                                    })
                                    .is_err()
                                {
                                    let _ = tx.try_send(Reply::new(id, ReplyStatus::Unknown));
                                } else {
                                    waiting.push(Waiting::Abort(id, tx, result));
                                }
                            }
                        }
                    }
                } else {
                    thread::sleep(Duration::from_millis(10));
                }
                waiting.retain_mut(|pending| match pending {
                    Waiting::Apply(id, rx) => match rx.try_recv() {
                        Ok(result) => {
                            shared
                                .lock()
                                .unwrap_or_else(|p| p.into_inner())
                                .finish(id, result.map(receipt).unwrap_or_else(declined));
                            notify();
                            false
                        }
                        Err(oneshot::error::TryRecvError::Empty) => true,
                        Err(_) => {
                            shared.lock().unwrap_or_else(|p| p.into_inner()).finish(
                                id,
                                ReplyStatus::Declined {
                                    code: DeclineCode::EngineUnavailable,
                                },
                            );
                            false
                        }
                    },
                    Waiting::Status(id, tx, rx) => match rx.try_recv() {
                        Ok(status) => {
                            let state = shared.lock().unwrap_or_else(|p| p.into_inner());
                            let status = match status {
                                SubmissionStatus::Accepted(r) => receipt(r),
                                SubmissionStatus::Declined(e) => declined(e),
                                SubmissionStatus::Pending => ReplyStatus::Pending,
                                SubmissionStatus::Unknown => state
                                    .entries
                                    .get(id)
                                    .map(|e| e.current.clone().unwrap_or(ReplyStatus::Pending))
                                    .unwrap_or(ReplyStatus::Unknown),
                            };
                            let _ = tx.try_send(Reply::new(id.clone(), status));
                            false
                        }
                        Err(oneshot::error::TryRecvError::Empty) => true,
                        Err(_) => {
                            let _ = tx.try_send(Reply::new(id.clone(), ReplyStatus::Unknown));
                            false
                        }
                    },
                    Waiting::Abort(id, tx, rx) => match rx.try_recv() {
                        Ok(result) => {
                            let mut state = shared.lock().unwrap_or_else(|p| p.into_inner());
                            let status = match result {
                                CancelOwnedResult::Cancelled => {
                                    state.finish(
                                        id,
                                        ReplyStatus::Declined {
                                            code: DeclineCode::BrowserFallback,
                                        },
                                    );
                                    ReplyStatus::Aborted
                                }
                                CancelOwnedResult::NotOwned
                                | CancelOwnedResult::AlreadyFinished => {
                                    match state.entries.get(id).and_then(|e| e.original.clone()) {
                                        Some(ReplyStatus::Accepted {
                                            job_id,
                                            created_new,
                                        }) => ReplyStatus::AlreadyAccepted {
                                            job_id,
                                            created_new,
                                        },
                                        _ => ReplyStatus::Unknown,
                                    }
                                }
                                CancelOwnedResult::Unknown => ReplyStatus::Unknown,
                            };
                            let _ = tx.try_send(Reply::new(id.clone(), status));
                            false
                        }
                        Err(oneshot::error::TryRecvError::Empty) => true,
                        Err(_) => {
                            let _ = tx.try_send(Reply::new(id.clone(), ReplyStatus::Unknown));
                            false
                        }
                    },
                });
                let mut state = shared.lock().unwrap_or_else(|p| p.into_inner());
                let expired: Vec<_> = state
                    .entries
                    .iter()
                    .filter_map(|(id, e)| {
                        (e.original.is_none()
                            && matches!(e.lease.state(), LeaseState::Expired | LeaseState::Revoked))
                        .then_some(id.clone())
                    })
                    .collect();
                for id in expired {
                    state.finish(
                        &id,
                        ReplyStatus::Declined {
                            code: DeclineCode::Expired,
                        },
                    );
                }
                let expired_media: Vec<_> = state
                    .media_entries
                    .iter()
                    .filter_map(|(id, entry)| {
                        (entry.result.is_none()
                            && matches!(
                                entry.lease.state(),
                                LeaseState::Expired | LeaseState::Revoked
                            ))
                        .then_some(id.clone())
                    })
                    .collect();
                for id in expired_media {
                    state.finish(
                        &id,
                        ReplyStatus::Declined {
                            code: DeclineCode::Expired,
                        },
                    );
                }
            }
            shared.lock().unwrap_or_else(|p| p.into_inner()).close();
        });
        Self {
            router: Arc::new(Router { state, work, wake }),
            stop,
            thread: Some(thread),
        }
    }
    pub fn set_policy(&self, policy: rusty_browser_bridge::policy::CapturePolicy) {
        let mut state = self.router.state.lock().unwrap_or_else(|p| p.into_inner());
        let rejected: Vec<_> = state
            .entries
            .iter()
            .filter(|(_, e)| {
                e.capture.source == CaptureSource::Automatic
                    && (!policy.allows(
                        e.capture.browser,
                        &e.capture.url,
                        e.capture.filename_hint.as_deref(),
                        e.capture.forced
                            && e.capture.policy_revision == policy.revision
                            && policy.keys.force != 0,
                    ) || e
                        .capture
                        .original_url
                        .as_ref()
                        .is_some_and(|u| policy.excluded(u)))
                    && e.lease.state() == LeaseState::Pending
            })
            .map(|(id, _)| id.clone())
            .collect();
        state.policy = policy;
        for id in rejected {
            if state
                .entries
                .get(&id)
                .is_some_and(|e| e.lease.revoke() == LeaseState::Accepted)
            {
                continue;
            }
            state.finish(
                &id,
                ReplyStatus::Declined {
                    code: DeclineCode::BrowserFallback,
                },
            );
        }
    }
    pub fn set_refresh_armed(&self, armed: bool) {
        self.router
            .state
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .refresh_armed = armed;
    }
    pub fn router(&self) -> Arc<dyn RequestRouter> {
        self.router.clone()
    }
    pub fn drain(&self) -> Vec<IncomingCapture> {
        self.router
            .state
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .inbox
            .drain(..)
            .collect()
    }
    pub fn drain_media(&self) -> Vec<IncomingMedia> {
        self.router
            .state
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .media_inbox
            .drain(..)
            .collect()
    }
    pub fn submit_media(
        &self,
        id: RequestId,
        request: rusty_engine::DownloadRequest,
        lease: CaptureLease,
    ) -> Result<(), CommandError> {
        if lease.state() != LeaseState::Pending {
            return Err(CommandError::Closed);
        }
        self.router
            .work
            .try_send(Work::Apply(id, SubmissionAction::Submit(request), lease))
            .map_err(|e| match e {
                mpsc::TrySendError::Full(_) => CommandError::QueueFull,
                mpsc::TrySendError::Disconnected(_) => CommandError::Closed,
            })
    }
    pub fn decline_media(&self, id: &RequestId, code: DeclineCode) {
        let mut state = self.router.state.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(entry) = state.media_entries.get(id) {
            if entry.lease.revoke() == LeaseState::Accepted {
                return;
            }
        }
        state.finish(id, ReplyStatus::Declined { code });
    }
    pub fn take_presentations(&self) -> Vec<rusty_engine::JobId> {
        self.router
            .state
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .presentations
            .drain(..)
            .collect()
    }
    pub fn take_activation(&self) -> bool {
        std::mem::take(
            &mut self
                .router
                .state
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .activate,
        )
    }
    pub fn subscribe_receipt(&self, id: &RequestId) -> Result<mpsc::Receiver<Reply>, CommandError> {
        let (tx, rx) = mpsc::sync_channel(1);
        let mut state = self.router.state.lock().unwrap_or_else(|p| p.into_inner());
        let entry = state.entries.get_mut(id).ok_or(CommandError::Closed)?;
        if let Some(status) = &entry.original {
            let _ = tx.try_send(Reply::new(id.clone(), status.clone()));
        } else if entry.waiters.len() < 8 {
            entry.waiters.push(tx);
        } else {
            return Err(CommandError::QueueFull);
        }
        Ok(rx)
    }
    pub fn submit(
        &self,
        id: RequestId,
        action: SubmissionAction,
        lease: CaptureLease,
        _reply: SyncSender<Reply>,
    ) -> Result<(), CommandError> {
        {
            let mut state = self.router.state.lock().unwrap_or_else(|p| p.into_inner());
            let denied = state.entries.get(&id).is_some_and(|e| {
                e.capture.source == CaptureSource::Automatic
                    && (!state.policy.allows(
                        e.capture.browser,
                        &e.capture.url,
                        e.capture.filename_hint.as_deref(),
                        e.capture.forced
                            && e.capture.policy_revision == state.policy.revision
                            && state.policy.keys.force != 0,
                    ) || e
                        .capture
                        .original_url
                        .as_ref()
                        .is_some_and(|u| state.policy.excluded(u)))
            });
            if denied {
                lease.revoke();
                state.finish(
                    &id,
                    ReplyStatus::Declined {
                        code: DeclineCode::BrowserFallback,
                    },
                );
                return Ok(());
            }
        }
        self.router
            .work
            .try_send(Work::Apply(id, action, lease))
            .map_err(|e| match e {
                mpsc::TrySendError::Full(_) => CommandError::QueueFull,
                mpsc::TrySendError::Disconnected(_) => CommandError::Closed,
            })
    }
    pub fn submit_refresh(
        &self,
        id: RequestId,
        job_id: JobId,
        candidate: RefreshCandidate,
        lease: CaptureLease,
    ) -> Result<(), CommandError> {
        if lease.state() != LeaseState::Pending {
            return Err(CommandError::Closed);
        }
        self.router
            .work
            .try_send(Work::Refresh(id, job_id, candidate, lease))
            .map_err(|e| match e {
                mpsc::TrySendError::Full(_) => CommandError::QueueFull,
                mpsc::TrySendError::Disconnected(_) => CommandError::Closed,
            })
    }
    pub fn decline(&self, id: &RequestId, code: DeclineCode) {
        let mut state = self.router.state.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(entry) = state.entries.get(id) {
            if entry.lease.revoke() == LeaseState::Accepted {
                return;
            }
        }
        state.finish(id, ReplyStatus::Declined { code });
    }
    pub fn close(&self) {
        self.router
            .state
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .close();
    }
}
impl Drop for BrowserController {
    fn drop(&mut self) {
        self.close();
        self.stop.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
fn receipt(r: SubmissionReceipt) -> ReplyStatus {
    ReplyStatus::Accepted {
        job_id: r.job_id.0,
        created_new: r.created_new,
    }
}
fn command_code(e: CommandError) -> DeclineCode {
    match e {
        CommandError::QueueFull => DeclineCode::Busy,
        CommandError::Closed => DeclineCode::AppClosing,
    }
}
fn declined(e: SubmissionError) -> ReplyStatus {
    ReplyStatus::Declined {
        code: match e {
            SubmissionError::Expired => DeclineCode::Expired,
            SubmissionError::Revoked => DeclineCode::BrowserFallback,
            SubmissionError::Busy => DeclineCode::Busy,
            SubmissionError::Closing => DeclineCode::AppClosing,
            SubmissionError::RecoveryRequired => DeclineCode::RecoveryRequired,
            _ => DeclineCode::InvalidRequest,
        },
    }
}
pub struct AppInstance {
    _mutex: OwnedHandle,
}
pub enum InstanceOutcome {
    Primary(AppInstance),
    Forwarded,
}
impl AppInstance {
    pub fn acquire_or_activate(
        session: &UserSession,
        activate: bool,
    ) -> Result<InstanceOutcome, crate::AppError> {
        use windows::{
            core::PCWSTR,
            Win32::{
                Foundation::{GetLastError, ERROR_ALREADY_EXISTS},
                System::Threading::CreateMutexW,
            },
        };
        let security = SecurityDescriptor::new(session)
            .map_err(|_| crate::AppError("Cannot secure application instance.".into()))?;
        let attributes = security.attributes();
        let name: Vec<u16> = session.mutex_name().encode_utf16().chain(Some(0)).collect();
        let handle = unsafe {
            let handle = CreateMutexW(Some(&attributes), false, PCWSTR(name.as_ptr()))?;
            let exists = GetLastError() == ERROR_ALREADY_EXISTS;
            (OwnedHandle(handle), exists)
        };
        if handle.1 {
            drop(handle.0);
            let mut client =
                PipeClient::connect(session, Duration::from_secs(10)).map_err(|_| {
                    crate::AppError(
                        "The existing Rusty instance is starting or unavailable.".into(),
                    )
                })?;
            client
                .request(if activate {
                    Operation::Activate
                } else {
                    Operation::Ping
                })
                .and_then(|replies| replies.recv())
                .map_err(|_| {
                    crate::AppError("Cannot activate the existing Rusty instance.".into())
                })?;
            Ok(InstanceOutcome::Forwarded)
        } else {
            Ok(InstanceOutcome::Primary(Self { _mutex: handle.0 }))
        }
    }
}
#[cfg(test)]
#[path = "browser_integration_tests.rs"]
mod tests;
