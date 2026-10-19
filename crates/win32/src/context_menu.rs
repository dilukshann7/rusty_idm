use crate::{
    commands::*, platform, presenter::ActionAvailability, settings::DoubleClickAction, AppError,
};
use rusty_engine::JobSnapshot;
use windows::Win32::{
    Foundation::{HWND, POINT},
    UI::WindowsAndMessaging::*,
};

struct Menu(HMENU);
impl Drop for Menu {
    fn drop(&mut self) {
        // Destroying the resource root also releases its owned submenus.
        unsafe {
            let _ = DestroyMenu(self.0);
        }
    }
}

pub fn can_redownload(job: &JobSnapshot) -> bool {
    job.state.is_terminal()
        && job.error.as_ref().is_none_or(|error| {
            error.retained_partial.is_none() && error.retained_original.is_none()
        })
}

pub fn preference_command(action: DoubleClickAction) -> u16 {
    match action {
        DoubleClickAction::Open => DOUBLE_CLICK_OPEN,
        DoubleClickAction::OpenFolder => DOUBLE_CLICK_FOLDER,
        DoubleClickAction::Properties => DOUBLE_CLICK_PROPERTIES,
    }
}

/// Returns the command directly so it stays attached to the captured JobId,
/// even if Windows dispatches selection notifications while tracking the menu.
pub fn show(
    owner: HWND,
    position: POINT,
    job: &JobSnapshot,
    actions: &ActionAvailability,
    double_click: DoubleClickAction,
) -> Result<Option<u16>, AppError> {
    unsafe {
        let root = Menu(LoadMenuW(
            Some(platform::instance()?),
            platform::resource(DOWNLOAD_CONTEXT_MENU),
        )?);
        let menu = GetSubMenu(root.0, 0);
        if menu.0.is_null() {
            return Err(AppError("Cannot load the download menu.".into()));
        }
        for (id, available) in [
            (OPEN_FILE, actions.open_file && job.destination.is_file()),
            (OPEN_WITH, actions.open_file && job.destination.is_file()),
            (
                OPEN_FOLDER,
                job.destination.parent().is_some_and(|path| path.is_dir()),
            ),
            (REDOWNLOAD, can_redownload(job)),
            (START, actions.start),
            (STOP, actions.stop),
            (REMOVE, actions.remove),
            (DETAILS, actions.details),
        ] {
            let _ = EnableMenuItem(
                menu,
                id as u32,
                MF_BYCOMMAND | if available { MF_ENABLED } else { MF_GRAYED },
            );
        }
        // Locate by its first command rather than tying this to resource positions.
        let choices = (0..GetMenuItemCount(Some(menu)))
            .map(|index| GetSubMenu(menu, index))
            .find(|submenu| {
                !submenu.0.is_null()
                    && GetMenuState(*submenu, DOUBLE_CLICK_OPEN as u32, MF_BYCOMMAND) != u32::MAX
            })
            .unwrap_or_default();
        if choices.0.is_null() {
            return Err(AppError("Cannot load the double-click menu.".into()));
        }
        CheckMenuRadioItem(
            choices,
            DOUBLE_CLICK_OPEN as u32,
            DOUBLE_CLICK_PROPERTIES as u32,
            preference_command(double_click) as u32,
            MF_BYCOMMAND.0,
        )?;
        let default = match double_click {
            DoubleClickAction::Open if actions.open_file => OPEN_FILE,
            DoubleClickAction::OpenFolder => OPEN_FOLDER,
            _ => DETAILS,
        };
        SetMenuDefaultItem(menu, default as u32, 0)?;
        let command = TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON,
            position.x,
            position.y,
            None,
            owner,
            None,
        )
        .0;
        Ok((command != 0).then_some(command as u16))
    }
}
