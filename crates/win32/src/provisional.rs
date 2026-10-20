//! One acknowledged provisional transfer owns File Info, including cancel/later.
use crate::{
    commands::*,
    dialogs::AddedDownload,
    native_form::{self as ui, Action, Form},
    platform::message,
    AppError,
};
use rusty_browser_bridge::{
    lease::{CaptureLease, LeaseState},
    protocol::{Reply, ReplyStatus, RequestId},
};
use rusty_engine::{submissions::*, Command, EngineClient, JobId, JobSnapshot, JobState, QueueId};
use windows::Win32::{Foundation::HWND, System::Com::CoCreateGuid, UI::WindowsAndMessaging::*};
pub enum Admission {
    Manual(
        RequestId,
        tokio::sync::oneshot::Receiver<Result<SubmissionReceipt, SubmissionError>>,
    ),
    Browser(RequestId, std::sync::mpsc::Receiver<Reply>),
}
impl Admission {
    fn id(&self) -> RequestId {
        match self {
            Self::Manual(id, _) | Self::Browser(id, _) => id.clone(),
        }
    }
    fn poll(&mut self) -> Result<Option<JobId>, String> {
        match self {
            Self::Manual(_, rx) => match rx.try_recv() {
                Ok(Ok(r)) => Ok(Some(r.job_id)),
                Ok(Err(_)) => {
                    Err("The engine declined this download. No download was accepted.".into())
                }
                Err(tokio::sync::oneshot::error::TryRecvError::Empty) => Ok(None),
                Err(_) => Err(
                    "Download admission acknowledgement was lost. Recovery state is preserved."
                        .into(),
                ),
            },
            Self::Browser(_, rx) => match rx.try_recv() {
                Ok(reply) => match reply.result {
                    ReplyStatus::Accepted { job_id, .. } => Ok(Some(JobId(job_id))),
                    ReplyStatus::Pending => Ok(None),
                    _ => Err("Browser handoff was declined or expired.".into()),
                },
                Err(std::sync::mpsc::TryRecvError::Empty) => Ok(None),
                Err(_) => Err("Browser handoff acknowledgement was lost.".into()),
            },
        }
    }
}
pub fn manual(
    engine: &EngineClient,
    download: &AddedDownload,
) -> Result<(CaptureLease, Admission), AppError> {
    let guid = unsafe { CoCreateGuid() }?;
    let compact = format!("{:032x}", guid.to_u128());
    let id = RequestId::parse(&format!(
        "{}-{}-{}-{}-{}",
        &compact[..8],
        &compact[8..12],
        &compact[12..16],
        &compact[16..20],
        &compact[20..]
    ))
    .map_err(|_| AppError("Cannot identify the download.".into()))?;
    let lease = CaptureLease::new(std::time::Instant::now() + std::time::Duration::from_secs(60));
    let (tx, rx) = tokio::sync::oneshot::channel();
    engine
        .try_external(ExternalCommand::Apply {
            id: id.clone(),
            action: SubmissionAction::Submit(download.request.clone()),
            lease: lease.clone(),
            reply: tx,
        })
        .map_err(|e| AppError(e.to_string()))?;
    Ok((lease, Admission::Manual(id, rx)))
}
#[derive(Clone, Copy)]
enum Pending {
    None,
    Cancel,
    Confirm,
    Pause(QueueId),
    Assign(QueueId),
    HoldConfirm(QueueId),
}
pub fn show(
    owner: HWND,
    engine: &EngineClient,
    download: &AddedDownload,
    lease: &CaptureLease,
    admission: Admission,
    choose_queue: bool,
) -> Result<(), AppError> {
    let request_id = admission.id();
    let result = show_inner(owner, engine, download, lease, admission, choose_queue);
    if result.is_err() {
        lease.revoke();
        let (tx, _) = tokio::sync::oneshot::channel();
        let _ = engine.try_external(ExternalCommand::CancelOwned {
            id: request_id,
            reply: tx,
        });
    }
    result
}
fn show_inner(
    owner: HWND,
    engine: &EngineClient,
    download: &AddedDownload,
    lease: &CaptureLease,
    mut admission: Admission,
    choose_queue: bool,
) -> Result<(), AppError> {
    let mut form = Form::new(owner, FILE_INFO_DIALOG, "Download File Info")?;
    let h = form.hwnd();
    ui::put(h, URL_EDIT, &download.request.display_url());
    ui::put(
        h,
        PATH_EDIT,
        &download.request.destination.to_string_lossy(),
    );
    ui::put(
        h,
        CATEGORY_COMBO,
        download.request.category.as_deref().unwrap_or("Automatic"),
    );
    ui::put(h, DESCRIPTION_EDIT, &download.request.description);
    ui::check(h, REMEMBER_PATH, download.remember_path);
    for id in [
        URL_EDIT,
        PATH_EDIT,
        CATEGORY_COMBO,
        CATEGORY_ADD,
        BROWSE,
        DESCRIPTION_EDIT,
        REMEMBER_PATH,
        1,
        DOWNLOAD_LATER,
    ] {
        ui::enabled(h, id, false);
    }
    ui::put(
        h,
        DIRECTORY_TEXT,
        "Receiving into a partial; waiting for admission...",
    );
    unsafe {
        SetTimer(Some(h), 1, 100, None);
    }
    form.show();
    let mut job = None;
    let mut pending = Pending::None;
    let mut inspect: Option<tokio::sync::oneshot::Receiver<Option<(JobSnapshot, bool)>>> = None;
    while let Some(actions) = form.next()? {
        if job.is_none() {
            match admission.poll() {
                Ok(Some(id)) => {
                    job = Some(id);
                    if matches!(pending, Pending::Cancel) {
                        send(engine, Command::Cancel(id))?;
                    } else {
                        ui::enabled(h, 1, true);
                        ui::enabled(h, DOWNLOAD_LATER, true);
                    }
                }
                Ok(None) => {}
                Err(error) => {
                    if matches!(pending, Pending::Cancel) && lease.state() != LeaseState::Accepted {
                        return Ok(());
                    }
                    return Err(AppError(error));
                }
            }
        }
        if let Some(rx) = &mut inspect {
            match rx.try_recv() {
                Ok(snapshot) => {
                    inspect = None;
                    let Some((snapshot, unconfirmed)) = snapshot else {
                        return Err(AppError("The download no longer exists.".into()));
                    };
                    ui::put(
                        h,
                        DIRECTORY_TEXT,
                        &format!(
                            "{} bytes received - {}",
                            snapshot.received,
                            if unconfirmed {
                                "waiting for confirmation"
                            } else {
                                "confirmed"
                            }
                        ),
                    );
                    let id = snapshot.id;
                    match pending {
                        Pending::Cancel if snapshot.state.is_terminal() => return Ok(()),
                        Pending::Confirm if !unconfirmed => return Ok(()),
                        Pending::Confirm => {
                            message(Some(h),"Cannot confirm download","The engine could not save confirmation. The partial remains unconfirmed; try again or cancel.",false);
                            pending = Pending::None;
                        }
                        Pending::Pause(queue)
                            if matches!(snapshot.state, JobState::Paused | JobState::Waiting) =>
                        {
                            send(engine, Command::AssignQueue(id, Some(queue)))?;
                            pending = Pending::Assign(queue);
                        }
                        Pending::Assign(queue) if snapshot.queue_id == Some(queue) => {
                            send(engine, Command::ConfirmInfo(id))?;
                            pending = Pending::HoldConfirm(queue);
                        }
                        Pending::Assign(_) => {
                            message(
                                Some(h),
                                "Cannot change queue",
                                "Queue membership was not acknowledged. The partial remains held.",
                                false,
                            );
                            pending = Pending::None;
                        }
                        Pending::HoldConfirm(queue)
                            if !unconfirmed && snapshot.queue_id == Some(queue) =>
                        {
                            return Ok(())
                        }
                        Pending::HoldConfirm(_) => {
                            message(
                                Some(h),
                                "Cannot confirm queue entry",
                                "The saved entry remains unconfirmed. Try again or cancel.",
                                false,
                            );
                            pending = Pending::None;
                        }
                        _ => {}
                    }
                    if snapshot.state.is_terminal() && !matches!(pending, Pending::Cancel) {
                        if let Some(error) = snapshot.error {
                            message(Some(h), "Download stopped", &error.message, false);
                        }
                        return Ok(());
                    }
                    if matches!(pending, Pending::None) {
                        ui::enabled(h, 1, true);
                        ui::enabled(h, DOWNLOAD_LATER, true);
                    }
                }
                Err(tokio::sync::oneshot::error::TryRecvError::Closed) => {
                    return Err(AppError(
                        "The engine closed; the provisional partial is retained for recovery."
                            .into(),
                    ))
                }
                Err(_) => {}
            }
        }
        if inspect.is_none() {
            if let Some(id) = job {
                let (tx, rx) = tokio::sync::oneshot::channel();
                send(engine, Command::Inspect(id, tx))?;
                inspect = Some(rx);
            }
        }
        for action in actions {
            if let Action::Command(_, id, _) = action {
                match id {
                    2 => {
                        pending = Pending::Cancel;
                        ui::enabled(h, 1, false);
                        ui::enabled(h, DOWNLOAD_LATER, false);
                        if let Some(job) = job {
                            send(engine, Command::Cancel(job))?;
                        } else if lease.revoke() != LeaseState::Accepted {
                            return Ok(());
                        }
                    }
                    1 if matches!(pending, Pending::None) => {
                        if let Some(id) = job {
                            send(engine, Command::ConfirmInfo(id))?;
                            pending = Pending::Confirm;
                            ui::enabled(h, 1, false);
                            ui::enabled(h, DOWNLOAD_LATER, false);
                            inspect = None;
                        }
                    }
                    DOWNLOAD_LATER if matches!(pending, Pending::None) => {
                        if let Some(id) = job {
                            let queue = if choose_queue {
                                let Some(queue) = crate::queue_picker::choose(h, engine)? else {
                                    continue;
                                };
                                queue
                            } else {
                                QueueId::MAIN
                            };
                            send(engine, Command::Pause(id))?;
                            pending = Pending::Pause(queue);
                            ui::enabled(h, 1, false);
                            ui::enabled(h, DOWNLOAD_LATER, false);
                            inspect = None;
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    // WM_QUIT/sign-out keeps the durable provisional record unconfirmed.
    Ok(())
}
fn send(engine: &EngineClient, command: Command) -> Result<(), AppError> {
    engine
        .try_send(command)
        .map_err(|e| AppError(e.to_string()))
}

pub fn wait_admission(
    owner: HWND,
    lease: &CaptureLease,
    mut admission: Admission,
) -> Result<Option<JobId>, AppError> {
    let mut form = Form::new(owner, ADMISSION_DIALOG, "Adding download")?;
    let h = form.hwnd();
    ui::put(h, ADMISSION_TEXT, "Saving download admission…");
    unsafe {
        SetTimer(Some(h), 1, 100, None);
    }
    form.show();
    while let Some(actions) = form.next()? {
        match admission.poll() {
            Ok(Some(id)) => return Ok(Some(id)),
            Ok(None) => {}
            Err(error) => return Err(AppError(error)),
        }
        for action in actions {
            if let Action::Command(_, 2, _) = action {
                if lease.revoke() != LeaseState::Accepted {
                    return Ok(None);
                }
            }
        }
    }
    lease.revoke();
    Ok(None)
}
