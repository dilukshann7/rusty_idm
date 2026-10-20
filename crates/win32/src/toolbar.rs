//! Native toolbar dropdowns and local sharing actions, outside window callbacks.
use crate::{
    commands::*,
    native_form::{self as ui, Action, Form},
    platform::{self, wide},
    AppError,
};
use rusty_engine::{QueueId, QueueSnapshot};
use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::{HWND, POINT},
        UI::WindowsAndMessaging::*,
    },
};

struct Menu(HMENU);
impl Drop for Menu {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyMenu(self.0);
        }
    }
}

pub fn queues(
    owner: HWND,
    anchor: POINT,
    command: u16,
    selected: QueueId,
    queues: &[QueueSnapshot],
) -> Result<Option<QueueId>, AppError> {
    let menu = Menu(unsafe { CreatePopupMenu() }?);
    for (index, queue) in queues.iter().enumerate() {
        let label = wide(&queue.name.replace('&', "&&"));
        let enabled = if command == QUEUE_START {
            !queue.stopping
        } else {
            queue.running || queue.stopping || queue.active > 0
        };
        let flags = MF_STRING
            | if enabled { MF_ENABLED } else { MF_GRAYED }
            | if queue.id == selected {
                MF_CHECKED
            } else {
                MF_UNCHECKED
            };
        unsafe {
            AppendMenuW(menu.0, flags, index + 1, PCWSTR(label.as_ptr()))?;
        }
    }
    let choice = unsafe {
        TrackPopupMenu(
            menu.0,
            TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON,
            anchor.x,
            anchor.y,
            None,
            owner,
            None,
        )
    }
    .0;
    Ok(choice
        .checked_sub(1)
        .and_then(|index| queues.get(index as usize))
        .map(|q| q.id))
}

pub fn share(owner: HWND) -> Result<(), AppError> {
    let mut form = Form::new(owner, SHARE_DIALOG, "Tell a Friend")?;
    let hwnd = form.hwnd();
    ui::put(hwnd, SHARE_LINK, "https://github.com/dilukshann7/rusty_idm");
    form.show();
    while let Some(actions) = form.next()? {
        for action in actions {
            if let Action::Command(_, id, _) = action {
                match id {
                    1 => {
                        match crate::clipboard::copy_text(
                            hwnd,
                            "https://github.com/dilukshann7/rusty_idm",
                        ) {
                            Ok(()) => ui::put(hwnd, 1, "Copied"),
                            Err(error) => {
                                platform::message(Some(hwnd), "Copy link", &error, false);
                            }
                        }
                    }
                    2 => return Ok(()),
                    _ => {}
                }
            }
        }
    }
    Ok(())
}
