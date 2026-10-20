//! Queue choices are engine snapshots; creation waits for durable acknowledgement.
use crate::{
    commands::*,
    native_form::{self as ui, Action, Form},
    platform::message,
    AppError,
};
use rusty_engine::{Command, EngineClient, QueueId, QueueSnapshot};
use windows::Win32::{Foundation::HWND, UI::WindowsAndMessaging::*};
pub fn choose(owner: HWND, engine: &EngineClient) -> Result<Option<QueueId>, AppError> {
    let mut form = Form::new(owner, QUEUE_PICKER, "Download queue")?;
    let h = form.hwnd();
    let (tx, mut query) = tokio::sync::oneshot::channel();
    engine
        .try_send(Command::QueryQueues(tx))
        .map_err(|e| AppError(e.to_string()))?;
    let mut rows: Vec<QueueSnapshot> = vec![];
    let mut create: Option<tokio::sync::oneshot::Receiver<Result<QueueId, String>>> = None;
    ui::enabled(h, 1, false);
    unsafe {
        SetTimer(Some(h), 1, 100, None);
    }
    form.show();
    while let Some(actions) = form.next()? {
        if let Ok(queues) = query.try_recv() {
            rows = queues;
            ui::choices(
                h,
                QUEUE_CHOICE,
                &rows.iter().map(|q| q.name.as_str()).collect::<Vec<_>>(),
                0,
            );
            ui::enabled(h, 1, !rows.is_empty());
        }
        if let Some(receiver) = &mut create {
            match receiver.try_recv() {
                Ok(result) => {
                    create = None;
                    ui::enabled(h, QUEUE_CREATE, true);
                    match result {
                        Ok(id) => return Ok(Some(id)),
                        Err(error) => {
                            message(Some(h), "Cannot create queue", &error, false);
                        }
                    }
                }
                Err(tokio::sync::oneshot::error::TryRecvError::Closed) => {
                    return Err(AppError(
                        "The engine closed before queue creation was acknowledged.".into(),
                    ))
                }
                Err(_) => {}
            }
        }
        for action in actions {
            if let Action::Command(_, id, _) = action {
                match id {
                    2 if create.is_none() => return Ok(None),
                    1 if create.is_none() => {
                        if let Some(queue) =
                            ui::selection(h, QUEUE_CHOICE).and_then(|n| rows.get(n))
                        {
                            return Ok(Some(queue.id));
                        }
                    }
                    QUEUE_CREATE if create.is_none() => {
                        if let Some(name) = crate::options_editors::text(
                            h,
                            "New queue",
                            "Enter the queue name.",
                            "",
                            80,
                        )? {
                            let (tx, rx) = tokio::sync::oneshot::channel();
                            match engine.try_send(Command::CreateQueueReply(name.trim().into(), tx))
                            {
                                Ok(()) => {
                                    create = Some(rx);
                                    ui::enabled(h, QUEUE_CREATE, false);
                                }
                                Err(e) => {
                                    message(Some(h), "Cannot create queue", &e.to_string(), false);
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(None)
}
