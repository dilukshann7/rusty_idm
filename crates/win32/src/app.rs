use crate::completion::{self, CompletionAction, CompletionWindows};
use crate::{
    bridge::{self, WakeTarget},
    commands::*,
    context_menu,
    controls::{scale, Controls},
    details::DetailsWindows,
    dialogs,
    duplicates::{self, DuplicateChoice},
    lifecycle::Lifecycle,
    platform::{self, message, resource, ComApartment},
    presenter::{Category, ViewModel},
    scheduler::{EditorAction, SchedulerWindow, Tab},
    settings::{self, DoubleClickAction, Settings},
    tray::{self, TrayIcon},
};
use crate::{
    browser_integration::{AppInstance, BrowserController, InstanceOutcome},
    download_intent::{DownloadIntent, ExistingAction},
};
use rusty_browser_bridge::{
    lease::{CaptureLease, LeaseState},
    protocol::DeclineCode,
    windows::{PipeServer, UserSession},
};
use rusty_engine::{spawn_engine, Command, CommandError, EngineHandle, Event};
use rusty_engine::refresh::RefreshCandidate;
use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    sync::Arc,
};
use windows::{
    core::w,
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM},
        Graphics::Gdi::{GetSysColorBrush, COLOR_WINDOW},
        UI::{Controls::*, HiDpi::*, WindowsAndMessaging::*},
    },
};

#[derive(Debug)]
pub struct AppError(pub String);
impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for AppError {}
impl From<windows::core::Error> for AppError {
    fn from(error: windows::core::Error) -> Self {
        Self(error.to_string())
    }
}
impl From<settings::SettingsError> for AppError {
    fn from(error: settings::SettingsError) -> Self {
        Self(error.to_string())
    }
}

enum Action {
    Command(u16),
    ContextMenu(POINT),
    QueueMenu(POINT),
    ToolbarQueue(u16),
    DoubleClick,
    Category(isize),
    AssociationsChanged,
    Selection,
    Layout,
    Dpi(u32, RECT),
    Engine,
    Browser,
    Close,
    Minimize,
    TrayMenu,
    RestoreTray,
    EndSession,
    Clipboard,
}
#[derive(Default)]
struct Signals {
    actions: RefCell<VecDeque<Action>>,
    alive: Cell<bool>,
    overflow: Cell<bool>,
    taskbar_created: Cell<u32>,
    session_shutdown: RefCell<Option<rusty_engine::EngineClient>>,
}
impl Signals {
    fn push(&self, action: Action) {
        let mut actions = self.actions.borrow_mut();
        let duplicate = actions.iter().any(|old| {
            matches!(
                (old, &action),
                (Action::Browser, Action::Browser)
                    | (Action::Engine, Action::Engine)
                    | (Action::Selection, Action::Selection)
                    | (Action::Layout, Action::Layout)
                    | (Action::Close, Action::Close)
                    | (Action::RestoreTray, Action::RestoreTray)
                    | (Action::TrayMenu, Action::TrayMenu)
                    | (Action::EndSession, Action::EndSession)
            )
        });
        if duplicate {
            return;
        }
        if actions.len() >= 128
            && matches!(
                action,
                Action::Command(_)
                    | Action::ContextMenu(_)
                    | Action::QueueMenu(_)
                    | Action::ToolbarQueue(_)
                    | Action::DoubleClick
            )
        {
            self.overflow.set(true);
            return;
        }
        actions.push_back(action);
    }
}
/// The box remains at a stable address until after WM_NCDESTROY. The callback
/// accesses only Signals, never AppState or engine/control owners.
struct MainWindow {
    hwnd: HWND,
    signals: Box<Signals>,
    wake: Arc<WakeTarget>,
}
impl MainWindow {
    fn new(wake: Arc<WakeTarget>) -> Result<Self, AppError> {
        unsafe {
            let instance = platform::instance()?;
            let class = WNDCLASSW {
                lpfnWndProc: Some(window_proc),
                hInstance: instance,
                lpszClassName: w!("RustyDownloadManager.Main"),
                hCursor: LoadCursorW(None, IDC_ARROW)?,
                hIcon: platform::app_icon()?,
                hbrBackground: GetSysColorBrush(COLOR_WINDOW),
                ..Default::default()
            };
            if RegisterClassW(&class) == 0 {
                return Err(windows::core::Error::from_thread().into());
            }
            let signals = Box::<Signals>::default();
            signals
                .taskbar_created
                .set(RegisterWindowMessageW(w!("TaskbarCreated")));
            let dpi = GetDpiForSystem().max(96);
            let hwnd = CreateWindowExW(
                WS_EX_CONTROLPARENT,
                class.lpszClassName,
                w!("Rusty Download Manager 0.1.0"),
                WS_OVERLAPPEDWINDOW | WS_CLIPCHILDREN,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                scale(1000, dpi),
                scale(540, dpi),
                None,
                None,
                Some(instance),
                Some((&*signals as *const Signals).cast()),
            )?;
            let window = Self {
                hwnd,
                signals,
                wake,
            };
            let menu = LoadMenuW(Some(instance), resource(MAIN_MENU))?;
            if let Err(error) = SetMenu(hwnd, Some(menu)) {
                let _ = DestroyMenu(menu);
                return Err(error.into());
            }
            window.wake.attach(hwnd);
            // Recovery polling also handles a rare failed PostMessage without losing events.
            if SetTimer(Some(hwnd), bridge::HOUSEKEEPING_TIMER, 250, None) == 0 {
                return Err(windows::core::Error::from_thread().into());
            }
            Ok(window)
        }
    }
    fn destroy(&self) {
        self.wake.disable();
        if self.signals.alive.get() {
            unsafe {
                let _ = KillTimer(Some(self.hwnd), bridge::HOUSEKEEPING_TIMER);
                let _ = DestroyWindow(self.hwnd);
            }
        }
    }
}
impl Drop for MainWindow {
    fn drop(&mut self) {
        self.destroy();
    }
}

struct AppState {
    browser: Option<BrowserController>,
    clipboard: crate::clipboard::Clipboard,
    engine: Option<EngineHandle>,
    view: ViewModel,
    controls: Controls,
    details: DetailsWindows,
    completions: CompletionWindows,
    scheduler: SchedulerWindow,
    scanner: crate::scanner::Scanner,
    pending_create: Option<u64>,
    pending_editor: Option<(rusty_engine::QueueId, Tab)>,
    refresh_target: Option<rusty_engine::JobId>,
    tray: TrayIcon,
    settings: Settings,
    category: Category,
    lifecycle: Lifecycle,
    shutdown_sent: bool,
    recovery_reported: bool,
    session_ending: bool,
}
impl AppState {
    fn resolve_download_intent(
        &mut self,
        window: &MainWindow,
        mut download: dialogs::AddedDownload,
        lease: Option<&CaptureLease>,
    ) -> Result<DownloadIntent, AppError> {
        let owner = window.hwnd;
        // Modal dialog callbacks only enqueue signals. Drain completed transfers
        // now so decisions use the engine's latest entry, not a stale row.
        self.events(window);
        if !self.lifecycle.accepts_submissions()
            || lease.is_some_and(|l| l.state() != LeaseState::Pending)
        {
            return Ok(DownloadIntent::Cancelled);
        }
        let existing = self
            .engine
            .as_ref()
            .and_then(|engine| engine.find_duplicate(&download.request));
        let disk = match rusty_engine::inspect_destination(&download.request.destination) {
            Ok(identity) => identity,
            Err(error) => {
                message(Some(owner), "Cannot use destination", &error.message, false);
                return Ok(DownloadIntent::Cancelled);
            }
        };
        let in_use = self.view.jobs.values().any(|job| {
            job.state.is_active()
                && duplicates::same_destination(&job.destination, &download.request.destination)
        });
        if existing.is_none() && disk.is_none() && !in_use {
            return Ok(DownloadIntent::New(download));
        }
        let choice = if let Some(choice) = self.settings.duplicate_choice {
            choice
        } else {
            let Some((choice, remember)) = dialogs::show_duplicate_with_lease(
                owner,
                &download.request.display_url(),
                DuplicateChoice::Existing,
                lease,
            )?
            else {
                return Ok(DownloadIntent::Cancelled);
            };
            if remember {
                let mut updated = self.settings.clone();
                updated.duplicate_choice = Some(choice);
                match settings::save(&updated) {
                    Ok(()) => self.settings = updated,
                    Err(error) => {
                        message(
                            Some(owner),
                            "Duplicate preference",
                            &format!(
                                "This choice will be used now, but could not be saved.\n\n{error}"
                            ),
                            false,
                        );
                    }
                };
            }
            choice
        };
        self.events(window);
        if !self.lifecycle.accepts_submissions()
            || lease.is_some_and(|l| l.state() != LeaseState::Pending)
        {
            return Ok(DownloadIntent::Cancelled);
        }
        if choice == DuplicateChoice::Overwrite {
            let target = existing
                .as_ref()
                .map(|s| s.destination.as_path())
                .unwrap_or(&download.request.destination);
            if !message(Some(owner),"Replace existing download?",&format!("Download again and replace an existing file only after the transfer succeeds?\n\n{}",target.display()),true){return Ok(DownloadIntent::Cancelled);}
        }
        match choice {
            DuplicateChoice::Numbered => {
                let reserved = self
                    .engine
                    .as_ref()
                    .map(EngineHandle::destinations)
                    .unwrap_or_default();
                match duplicates::numbered_destination(&download.request.destination, &reserved) {
                    Ok(path) => {
                        download.request.destination = path;
                        download.request.allow_duplicate = true;
                        download.request.replacement = rusty_engine::ReplacementPolicy::Deny;
                        Ok(DownloadIntent::New(download))
                    }
                    Err(error) => {
                        message(
                            Some(owner),
                            "Cannot number filename",
                            &error.to_string(),
                            false,
                        );
                        Ok(DownloadIntent::Cancelled)
                    }
                }
            }
            DuplicateChoice::Overwrite if existing.is_some() => {
                let existing = existing.expect("matched existing entry");
                if !existing.state.is_terminal()
                    && existing.state != rusty_engine::JobState::Waiting
                {
                    message(
                        Some(owner),
                        "Download in use",
                        "Stop this download before replacing it, or select a numbered copy.",
                        false,
                    );
                    return Ok(DownloadIntent::Cancelled);
                }
                let replacement = rusty_engine::inspect_destination(&existing.destination)
                    .map_err(|e| AppError(e.message))?
                    .map(rusty_engine::ReplacementPolicy::Approved)
                    .unwrap_or(rusty_engine::ReplacementPolicy::Deny);
                Ok(DownloadIntent::RestartApproved {
                    job_id: existing.id,
                    replacement,
                })
            }
            DuplicateChoice::Overwrite => {
                if self.view.jobs.values().any(|job| {
                    job.state.is_active()
                        && duplicates::same_destination(
                            &job.destination,
                            &download.request.destination,
                        )
                }) {
                    message(Some(owner), "Filename in use", "Another active download uses this filename. Stop it first, or choose a numbered copy.", false);
                    return Ok(DownloadIntent::Cancelled);
                }
                download.request.replacement =
                    match rusty_engine::inspect_destination(&download.request.destination) {
                        Ok(Some(identity)) => rusty_engine::ReplacementPolicy::Approved(identity),
                        Ok(None) => rusty_engine::ReplacementPolicy::Deny,
                        Err(error) => {
                            message(Some(owner), "Cannot replace file", &error.message, false);
                            return Ok(DownloadIntent::Cancelled);
                        }
                    };
                download.request.allow_duplicate = true;
                Ok(DownloadIntent::New(download))
            }
            DuplicateChoice::Existing => {
                if let Some(existing) = existing {
                    let latest = self
                        .engine
                        .as_ref()
                        .and_then(|engine| engine.find_duplicate(&download.request))
                        .unwrap_or(existing);
                    if latest.state == rusty_engine::JobState::Completed
                        && !latest.destination.is_file()
                    {
                        return Ok(DownloadIntent::RestartApproved {
                            job_id: latest.id,
                            replacement: rusty_engine::ReplacementPolicy::Deny,
                        });
                    }
                    let action = if !latest.state.is_terminal() || latest.state.can_retry() {
                        ExistingAction::Activate
                    } else {
                        ExistingAction::Show
                    };
                    return Ok(DownloadIntent::Existing {
                        job_id: latest.id,
                        action,
                    });
                } else if download.request.destination.is_file() {
                    message(
                        Some(owner),
                        "Download complete",
                        &format!(
                            "The file is already available.\n\n{}",
                            download.request.destination.display()
                        ),
                        false,
                    );
                } else {
                    message(Some(owner), "Filename in use", "Another download is using this filename. Choose a numbered copy to add this link.", false);
                }
                Ok(DownloadIntent::Cancelled)
            }
        }
    }
    fn command(&self, owner: HWND, command: Command) -> bool {
        if let Some(engine) = &self.engine {
            if let Err(error) = engine.try_send(command) {
                message(
                    Some(owner),
                    "Command unavailable",
                    &error.to_string(),
                    false,
                );
                return false;
            }
            return true;
        }
        false
    }
    fn close(&mut self, owner: HWND) {
        if self.lifecycle == Lifecycle::Open && self.scheduler.request_exit() {
            return;
        }
        self.close_after_editor(owner);
    }
    fn close_after_editor(&mut self, owner: HWND) {
        if self.lifecycle != Lifecycle::Open {
            return;
        }
        let active = self.view.active();
        let accepted = !active || message(Some(owner), "Save downloads and exit?", "Exit Rusty? Unfinished downloads keep their saved partials and return held after restart.", true);
        if self.lifecycle.begin_close(active, accepted) {
            if let Some(browser) = &self.browser {
                browser.close();
            }
            self.send_shutdown(owner);
        }
    }
    fn send_shutdown(&mut self, owner: HWND) {
        if self.shutdown_sent {
            return;
        }
        if let Some(engine) = &self.engine {
            match engine.try_send(Command::Shutdown) {
                Ok(()) => self.shutdown_sent = true,
                Err(CommandError::QueueFull) => {} // Timer retries; the UI keeps pumping.
                Err(CommandError::Closed) => self.shutdown_sent = true,
            }
        }
        self.controls.actions(owner, &self.view, true);
    }
    fn events(&mut self, window: &MainWindow) {
        let events = self
            .engine
            .as_ref()
            .map(|e| e.drain_events())
            .unwrap_or_default();

        for event in events {
            match &event {
                Event::JobChanged(snapshot) => {
                    if self.refresh_target == Some(snapshot.id)
                        && matches!(snapshot.state, rusty_engine::JobState::Running | rusty_engine::JobState::Completed | rusty_engine::JobState::Waiting)
                    {
                        self.refresh_target = None;
                        if let Some(browser) = &self.browser { browser.set_refresh_armed(false); }
                    }
                    let newly_completed = !snapshot.restored
                        && snapshot.state == rusty_engine::JobState::Completed
                        && self.view.jobs.get(&snapshot.id).map_or(true, |previous| {
                            previous.state != rusty_engine::JobState::Completed
                        });
                    if self.lifecycle == Lifecycle::Open {
                        if let Err(error) = self.details.observe(window.hwnd, snapshot) {
                            message(
                                Some(window.hwnd),
                                "Download status",
                                &error.to_string(),
                                false,
                            );
                        }
                    } else {
                        self.details.update(snapshot);
                    }
                    if snapshot.state != rusty_engine::JobState::Completed {
                        self.scanner.invalidate(snapshot.id);
                    }
                    if newly_completed
                        && self.lifecycle == Lifecycle::Open
                        && self.settings.configuration.scanner.enabled
                    {
                        self.scanner.enqueue(
                            snapshot.clone(),
                            self.settings.configuration.scanner.clone(),
                        );
                    } else if newly_completed
                        && self.lifecycle == Lifecycle::Open
                        && self.settings.show_completion_dialog
                    {
                        if let Err(error) = self.completions.show(window.hwnd, snapshot) {
                            message(
                                Some(window.hwnd),
                                "Download complete",
                                &error.to_string(),
                                false,
                            );
                        }
                    }
                }
                Event::QueuesChanged(_) => {}
                Event::QueueCreated { request, result } => {
                    if self.pending_create == Some(*request) {
                        self.pending_create = None;
                        self.scheduler.creating(false);
                        match result {
                            Ok(id) => self.pending_editor = Some((*id, Tab::Schedule)),
                            Err(error) => {
                                if self.lifecycle == Lifecycle::Open {
                                    message(Some(window.hwnd), "Cannot create queue", error, false);
                                }
                            }
                        }
                    }
                }
                Event::QueueConfigured { request, result } => {
                    self.scheduler.result(*request, result);
                    if let Err(error) = result {
                        if self.lifecycle == Lifecycle::Open {
                            message(Some(window.hwnd), "Queue settings", error, false);
                        }
                    }
                }
                Event::JobRemoved(id) => {
                    if self.refresh_target == Some(*id) {
                        self.refresh_target = None;
                        if let Some(browser) = &self.browser { browser.set_refresh_armed(false); }
                    }
                    self.details.remove(*id);
                    self.scanner.removed(*id);
                }
                Event::CommandRejected { message: text } => {
                    if text == "Too many queue commands await acknowledgement." {
                        self.pending_create = None;
                        self.scheduler.creating(false);
                        self.scheduler.rejected();
                    }
                    message(Some(window.hwnd), "Download command", text, false);
                }
                Event::ShutdownComplete => {
                    self.lifecycle.engine_stopped();
                }
            }
            self.view.apply(event);
        }
        if self.lifecycle == Lifecycle::Open {
            if let Some((id, tab)) = self
                .pending_editor
                .filter(|(id, _)| self.view.queues.contains_key(id))
            {
                self.pending_editor = None;
                if let Err(error) = self.scheduler.open(window.hwnd, &self.view, id, tab) {
                    message(Some(window.hwnd), "Queue editor", &error.to_string(), false);
                }
            }
        }
        for command in self.details.take_commands() {
            if self.lifecycle == Lifecycle::Open {
                self.command(window.hwnd, command);
            }
        }
        self.details.prune();
        {
            self.controls.poll_icons(
                window.hwnd,
                &self.view,
                self.category,
                self.lifecycle != Lifecycle::Open,
            );
        }
        for (snapshot, result) in self.scanner.drain() {
            if self.lifecycle == Lifecycle::Open
                && self.view.jobs.get(&snapshot.id).is_some_and(|s| {
                    s.state == rusty_engine::JobState::Completed
                        && s.destination == snapshot.destination
                })
            {
                if let Err(error) = result {
                    message(Some(window.hwnd), "Virus check", &error, false);
                }
                if self.settings.show_completion_dialog {
                    if let Err(error) = self.completions.show(window.hwnd, &snapshot) {
                        message(
                            Some(window.hwnd),
                            "Download complete",
                            &error.to_string(),
                            false,
                        );
                    }
                }
            }
        }
        if self.lifecycle == Lifecycle::Closing {
            self.send_shutdown(window.hwnd);
        }
        if self.lifecycle == Lifecycle::Closed {
            if !self.recovery_reported {
                self.recovery_reported = true;
                if let Some(report) = self.view.recovery_notice().filter(|_| !self.session_ending) {
                    if let Err(error) = dialogs::show_recovery_notice(window.hwnd, &report) {
                        message(
                            Some(window.hwnd),
                            "Partial files remain on disk",
                            &format!("{report}\n\nCould not open the recovery dialog: {error}"),
                            false,
                        );
                    }
                }
            }
            self.tray.remove();
            self.completions.close_all();
            self.scheduler.close_all();
            self.details.close_all();
            window.destroy();
        }
    }
    fn handle(&mut self, window: &MainWindow, action: Action) -> Result<(), AppError> {
        let owner = window.hwnd;
        match action {
            Action::Engine => {
                self.events(window);
                self.browser_events(window)?;
            }
            Action::Clipboard if self.lifecycle.accepts_submissions() => {
                if let Some(address) = self.clipboard.take_url() {
                    let policy = &self.settings.configuration.capture;
                    let name = crate::download_form::suggested_filename(&address).ok();
                    let extension = name
                        .as_deref()
                        .and_then(|n| std::path::Path::new(n).extension())
                        .and_then(|e| e.to_str())
                        .unwrap_or("")
                        .to_ascii_lowercase();
                    if !policy.excluded(&address) && policy.extensions.contains(&extension) {
                        let client = self.engine.as_ref().expect("running engine").client();
                        if let Some(download) =
                            dialogs::from_address(owner, &self.settings, &address, &client)?
                        {
                            let intent = self.resolve_download_intent(window, download, None)?;
                            self.execute_manual_intent(owner, intent)?;
                        }
                    }
                }
            }
            Action::Browser => self.browser_events(window)?,
            Action::Close => {
                if self.tray.available() {
                    unsafe {
                        let _ = ShowWindow(owner, SW_HIDE);
                    }
                } else {
                    self.close(owner);
                }
            }
            Action::Minimize => {
                if self.tray.available() {
                    unsafe {
                        let _ = ShowWindow(owner, SW_HIDE);
                    }
                }
            }
            Action::EndSession => {
                self.session_ending = true;
                if self.lifecycle.begin_close(self.view.active(), true) {
                    if let Some(browser) = &self.browser {
                        browser.close();
                    }
                    self.send_shutdown(owner);
                }
            }
            Action::RestoreTray => {
                if !self.tray.install() {
                    platform::bring_to_front(owner);
                }
            }
            Action::TrayMenu if self.lifecycle.accepts_submissions() => {
                if let Some(command) = self.tray.show_menu()? {
                    self.handle(window, Action::Command(command))?;
                }
            }
            Action::Layout => self.controls.layout(owner),
            Action::Dpi(dpi, rectangle) => {
                self.controls.change_dpi(dpi)?;
                unsafe {
                    SetWindowPos(
                        owner,
                        None,
                        rectangle.left,
                        rectangle.top,
                        rectangle.right - rectangle.left,
                        rectangle.bottom - rectangle.top,
                        SWP_NOACTIVATE | SWP_NOZORDER,
                    )?;
                }
                self.controls.layout(owner);
            }
            Action::AssociationsChanged => {
                self.controls.invalidate_associations()?;
            }
            Action::Category(index) => {
                let Some(category) = self.controls.category_for(index) else {
                    return Ok(());
                };
                self.category = category;
                self.controls.refresh(
                    owner,
                    &self.view,
                    category,
                    self.lifecycle != Lifecycle::Open,
                );
            }
            Action::Selection => {
                self.controls
                    .actions(owner, &self.view, self.lifecycle != Lifecycle::Open)
            }
            Action::Command(EXIT) => self.close(owner),
            Action::ToolbarQueue(command) if self.lifecycle.accepts_submissions() => {
                if let Some(anchor) = self.controls.queue_button_anchor(command) {
                    let queues: Vec<_> = self.view.queues.values().cloned().collect();
                    let selected = match self.category {
                        Category::Queue(id) => id,
                        _ => rusty_engine::QueueId::MAIN,
                    };
                    let choice = crate::toolbar::queues(owner, anchor, command, selected, &queues)?;
                    self.events(window);
                    if self.lifecycle.accepts_submissions() {
                        if let Some(id) = choice.filter(|id| self.view.queues.contains_key(id)) {
                            self.queue_action(window, id, command)?;
                        }
                    }
                }
            }
            Action::QueueMenu(position) if self.lifecycle.accepts_submissions() => {
                if let Some((queue, position)) = self.controls.queue_context_target(position) {
                    let command = context_menu::show_queue(
                        owner,
                        position,
                        queue.and_then(|id| self.view.queues.get(&id)),
                    )?;
                    self.events(window);
                    if let Some(command) = command.filter(|_| self.lifecycle.accepts_submissions())
                    {
                        if command == QUEUE_NEW {
                            self.new_queue(window)?;
                        } else if let Some(id) =
                            queue.filter(|id| self.view.queues.contains_key(id))
                        {
                            self.queue_action(window, id, command)?;
                        }
                    }
                }
            }
            Action::ContextMenu(position) if self.lifecycle.accepts_submissions() => {
                if let Some((selected, position)) = self.controls.context_target(position) {
                    let queues: Vec<_> = self.view.queues.values().cloned().collect();
                    let command = self
                        .view
                        .jobs
                        .get(&selected)
                        .map(|job| {
                            context_menu::show(
                                owner,
                                position,
                                job,
                                &self.view.actions(Some(selected)),
                                self.settings.double_click_action,
                                &queues,
                            )
                        })
                        .transpose()?
                        .flatten();
                    // Timers only queued signals during TrackPopupMenu. Apply the
                    // latest engine state before acting on this captured job.
                    self.events(window);
                    if self.lifecycle.accepts_submissions() {
                        if let Some(command) = command {
                            if command >= QUEUE_DYNAMIC_BASE {
                                if let Some(queue) =
                                    queues.get((command - QUEUE_DYNAMIC_BASE) as usize)
                                {
                                    self.command(
                                        owner,
                                        Command::AssignQueue(selected, Some(queue.id)),
                                    );
                                }
                            } else {
                                self.download_command(window, Some(selected), command)?;
                            }
                        }
                    }
                }
            }
            Action::DoubleClick if self.lifecycle.accepts_submissions() => {
                let selected = self.controls.selected();
                let command = match self.settings.double_click_action {
                    DoubleClickAction::Open if self.view.actions(selected).open_file => OPEN_FILE,
                    DoubleClickAction::OpenFolder => OPEN_FOLDER,
                    _ => DETAILS,
                };
                self.download_command(window, selected, command)?;
            }
            Action::Command(id) if self.lifecycle.accepts_submissions() => {
                let selected = self.controls.selected();
                let actions = self.view.actions(selected);
                match id {
                    QUEUE_NEW => self.new_queue(window)?,
                    QUEUE_RENAME | QUEUE_DELETE | QUEUE_START | QUEUE_STOP | QUEUE_ONE
                    | QUEUE_TWO | QUEUE_EDIT | QUEUE_SCHEDULE => {
                        let queue = match self.category {
                            Category::Queue(id) => id,
                            _ => rusty_engine::QueueId::MAIN,
                        };
                        self.queue_action(window, queue, id)?;
                    }
                    DELETE_COMPLETED => {
                        self.command(owner, Command::RemoveCompleted);
                    }
                    TELL_FRIEND => crate::toolbar::share(owner)?,
                    SHOW_MANAGER => platform::bring_to_front(owner),
                    ADD => {
                        if let Some(prepared) =
                            dialogs::prepare_add_download(owner, &self.settings)?
                        {
                            let intent = self.prepared_intent(window, prepared, None)?;
                            self.execute_manual_intent(owner, intent)?;
                        }
                    }
                    ADD_BATCH => self.add_batch(window)?,
                    OPTIONS => {
                        if let Some(settings) = dialogs::show_options(
                            owner,
                            &self.settings,
                            self.engine.as_ref().expect("running engine").client(),
                        )? {
                            if let Some(browser) = &self.browser {
                                browser.set_policy(settings.configuration.capture.clone());
                            }
                            self.details
                                .set_preferences(settings.configuration.progress.clone());
                            self.clipboard.adopt(settings.configuration.clipboard);
                            self.view.categories = settings.configuration.categories.clone();
                            self.settings = settings;
                            self.controls
                                .refresh(owner, &self.view, self.category, false);
                            self.completions
                                .set_show_again(self.settings.show_completion_dialog);
                        }
                    }
                    STOP_ALL if actions.stop_all => {
                        self.command(owner, Command::CancelAll);
                    }
                    CATEGORY_ALL => {
                        self.controls.select_all_category();
                        self.category = Category::All;
                        self.controls
                            .refresh(owner, &self.view, self.category, false);
                    }
                    ABOUT => {
                        message(Some(owner), "About Rusty Download Manager", "Rusty Download Manager pre-alpha\n\nNative Windows controls and single-stream HTTP/HTTPS downloads.\nTwo simultaneous downloads. Unfinished jobs return held after restart.\n\nPause/resume, speed limits, guarded replacement and browser capture.\nClose the manager to keep Rusty in the tray. Use Exit to save unfinished jobs.\nBrowser integration requires one-time extension setup.", false);
                    }
                    _ => self.download_command(window, selected, id)?,
                }
            }
            _ => {}
        }
        Ok(())
    }
    fn begin_refresh(
        &mut self,
        window: &MainWindow,
        id: rusty_engine::JobId,
    ) -> Result<(), AppError> {
        let owner = window.hwnd;
        if self.refresh_target == Some(id) {
            self.refresh_target = None;
            if let Some(browser) = &self.browser { browser.set_refresh_armed(false); }
            message(Some(owner), "Refresh cancelled", "Browser capture is no longer waiting for this download.", false);
            return Ok(());
        }
        self.refresh_target = None;
        if let Some(browser) = &self.browser { browser.set_refresh_armed(false); }
        let (reply, result) = std::sync::mpsc::sync_channel(1);
        if !self.command(owner, Command::PrepareRefresh(id, reply)) { return Ok(()); }
        let source_page = match result.recv_timeout(std::time::Duration::from_secs(15)) {
            Ok(Ok(page)) => page,
            Ok(Err(error)) => { message(Some(owner), "Cannot refresh address", &error, false); return Ok(()); }
            Err(_) => { message(Some(owner), "Cannot refresh address", "Timed out while saving the partial. Try again after the download is paused.", false); return Ok(()); }
        };
        self.events(window);
        match crate::refresh_ui::choose(owner, source_page.clone())? {
            crate::refresh_ui::RefreshChoice::Cancel => {}
            crate::refresh_ui::RefreshChoice::WaitForBrowser => {
                if let Some(browser) = &self.browser {
                    self.refresh_target = Some(id);
                    browser.set_refresh_armed(true);
                    message(Some(owner), "Waiting for new address", "Start the same download in your browser. Rusty will ask before using the new link. Choose Refresh download address again to cancel waiting.", false);
                } else {
                    message(Some(owner), "Browser unavailable", "Browser capture is unavailable. Use Refresh download address and paste a new URL.", false);
                }
            }
            crate::refresh_ui::RefreshChoice::Paste(url) => {
                let (reply, _) = tokio::sync::oneshot::channel();
                self.command(owner, Command::RefreshAddress {
                    job_id: id,
                    candidate: RefreshCandidate { url, source_page, request_context: Vec::new() },
                    reply,
                });
            }
        }
        Ok(())
    }
    fn download_command(
        &mut self,
        window: &MainWindow,
        selected: Option<rusty_engine::JobId>,
        command: u16,
    ) -> Result<(), AppError> {
        let owner = window.hwnd;
        if let Some(preference) = match command {
            DOUBLE_CLICK_OPEN => Some(DoubleClickAction::Open),
            DOUBLE_CLICK_FOLDER => Some(DoubleClickAction::OpenFolder),
            DOUBLE_CLICK_PROPERTIES => Some(DoubleClickAction::Properties),
            _ => None,
        } {
            let mut settings = self.settings.clone();
            settings.double_click_action = preference;
            settings::save(&settings)?;
            self.settings = settings;
            return Ok(());
        }
        let Some(id) = selected else {
            return Ok(());
        };
        let Some(job) = self.view.jobs.get(&id) else {
            return Ok(());
        };
        if command == REFRESH_ADDRESS {
            if job.refreshable { self.begin_refresh(window, id)?; }
            return Ok(());
        }
        let actions = self.view.actions(Some(id));
        match command {
            QUEUE_ADD=>{self.command(owner,Command::AssignQueue(id,Some(rusty_engine::QueueId::MAIN)));},
            QUEUE_REMOVE=>{self.command(owner,Command::AssignQueue(id,None));},
            QUEUE_UP|QUEUE_DOWN=>{self.command(owner,Command::MoveQueueEntry(id,command==QUEUE_UP));},
            CREDENTIALS=>{if let Some((login,password))=crate::queue_ui::credentials(owner)?{self.command(owner,Command::SupplyCredentials(id,login,password));}},
            RESTART_REQUIRED=>{if message(Some(owner),"Restart download?",&format!("Restart {} from zero?\n\nSave as: {}\n\nVerified saved bytes will be discarded. Modified partials will be preserved. If a file already exists at this destination, this approves replacing it only after a successful download.",job.filename,job.destination.display()),true){self.command(owner,Command::ConfirmRestart(id));}},
            CATEGORY_EDIT=>{if let Some(category)=crate::queue_ui::name(owner,"Category (Automatic or a category name)",job.category.as_deref().unwrap_or("Automatic"))?{self.command(owner,Command::SetCategory(id,(!category.eq_ignore_ascii_case("Automatic")).then_some(category)));}},
            START if job.readiness==rusty_engine::EntryReadiness::ConfirmationRequired => {if message(Some(owner),"Confirm saved download",&format!("Continue the unconfirmed download {}?",job.filename),true){self.command(owner,Command::ConfirmInfo(id));self.command(owner,Command::Activate(id));}}
            START if actions.start => {self.command(owner,Command::Activate(id));}
            STOP if actions.stop => {
                self.command(owner, Command::Cancel(id));
            }
            REMOVE if actions.remove => {
                self.command(owner, Command::Remove(id));
            }
            RETRY if actions.retry => {
                self.command(owner, Command::Retry(id));
            }
            DETAILS => {
                self.details.show_or_update(owner, job)?;
            }
            OPEN_FILE if actions.open_file => {
                if !self.scanner.allow_open(owner,&job.destination){return Ok(());}
                completion::open_completed(owner, &job.destination, completion::OpenAction::Open)?;
            }
            OPEN_WITH if actions.open_file => {
                if !self.scanner.allow_open(owner,&job.destination){return Ok(());}
                completion::open_completed(
                    owner,
                    &job.destination,
                    completion::OpenAction::OpenWith,
                )?;
            }
            OPEN_FOLDER => {
                completion::open_completed(owner, &job.destination, completion::OpenAction::OpenFolder)?;
            }
            REDOWNLOAD if context_menu::can_redownload(job) => {
                if message(Some(owner), "Redownload?",
                    &format!("Download {} again from the beginning?\n\nAn existing destination file will be replaced only if the download succeeds.", job.filename), true)
                {
                    self.events(window);
                    if self.lifecycle.accepts_submissions()
                        && self.view.jobs.get(&id).is_some_and(context_menu::can_redownload)
                    {
                        self.command(owner, Command::Restart(id));
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }
}

pub fn run() -> Result<(), AppError> {
    // Standard installer mutex: setup refuses to replace running executables.
    let _installer_guard = rusty_browser_bridge::windows::OwnedHandle(unsafe {
        windows::Win32::System::Threading::CreateMutexW(
            None,
            false,
            w!("Local\\RustyDownloadManager.Running"),
        )?
    });
    let background = std::env::args_os()
        .skip(1)
        .any(|argument| argument == "--background");
    let session = UserSession::current()
        .map_err(|_| AppError("Cannot determine browser session identity.".into()))?;
    let _instance = match AppInstance::acquire_or_activate(&session, !background)? {
        InstanceOutcome::Primary(instance) => instance,
        InstanceOutcome::Forwarded => return Ok(()),
    };
    let _com = ComApartment::new()?;
    unsafe {
        let controls = INITCOMMONCONTROLSEX {
            dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
            dwICC: ICC_BAR_CLASSES
                | ICC_TREEVIEW_CLASSES
                | ICC_LISTVIEW_CLASSES
                | ICC_PROGRESS_CLASS
                | ICC_TAB_CLASSES
                | ICC_STANDARD_CLASSES
                | ICC_DATE_CLASSES
                | ICC_UPDOWN_CLASS,
        };
        if !InitCommonControlsEx(&controls).as_bool() {
            return Err(AppError(
                "Windows common controls could not be initialized.".into(),
            ));
        }
    }
    let (settings, warning) = match settings::load() {
        Ok(settings) => (settings, None),
        Err(error) => (settings::defaults()?, Some(error.to_string())),
    };
    let wake = Arc::new(WakeTarget::new());
    let window = MainWindow::new(wake.clone())?;
    let controls = Controls::new(window.hwnd)?;
    let engine_wake = wake.clone();
    let engine = spawn_engine(Arc::new(move || engine_wake.wake()));
    *window.signals.session_shutdown.borrow_mut() = Some(engine.client());
    let browser = BrowserController::new(engine.client(), Arc::new(move || wake.wake_browser()));
    browser.set_policy(settings.configuration.capture.clone());
    let pipe = match PipeServer::start(session, browser.router()) {
        Ok(pipe) => pipe,
        Err(_) => {
            drop(browser);
            let _ = engine.join();
            return Err(AppError(
                "Cannot own the browser integration endpoint.".into(),
            ));
        }
    };
    let mut app = AppState {
        browser: Some(browser),
        clipboard: crate::clipboard::Clipboard::new(window.hwnd),
        engine: Some(engine),
        view: ViewModel::default(),
        controls,
        details: DetailsWindows::default(),
        completions: CompletionWindows::default(),
        scheduler: SchedulerWindow::default(),
        scanner: crate::scanner::Scanner::default(),
        pending_create: None,
        pending_editor: None,
        refresh_target: None,
        tray: TrayIcon::new(window.hwnd),
        settings,
        category: Category::All,
        lifecycle: Lifecycle::Open,
        shutdown_sent: false,
        recovery_reported: false,
        session_ending: false,
    };
    if let Err(error) = app
        .clipboard
        .configure(app.settings.configuration.clipboard)
    {
        message(Some(window.hwnd), "Clipboard", &error, false);
        app.settings.configuration.clipboard = false;
    }
    app.view.categories = app.settings.configuration.categories.clone();
    app.details
        .set_preferences(app.settings.configuration.progress.clone());
    app.controls.layout(window.hwnd);
    app.controls
        .refresh(window.hwnd, &app.view, app.category, false);
    let tray_ready = app.tray.install();
    unsafe {
        let _ = ShowWindow(
            window.hwnd,
            if background && tray_ready {
                SW_HIDE
            } else {
                SW_SHOW
            },
        );
        let _ = windows::Win32::Graphics::Gdi::UpdateWindow(window.hwnd);
    }
    if let Some(warning) = warning {
        message(Some(window.hwnd), "Settings warning", &format!("{warning}\n\nUsing Windows Downloads for this session. Save a folder through Options to update settings."), false);
    }
    let result = message_loop(&window, &mut app);
    if let Some(browser) = &app.browser {
        browser.close();
    }
    drop(pipe);
    drop(app.browser.take());
    app.tray.remove();
    app.completions.close_all();
    app.scheduler.close_all();
    app.details.close_all();
    window.destroy();
    // On an exceptional message-loop exit, dropping the sender in join() also
    // requests actor cleanup. No join is ever executed in a native callback.
    let joined = app
        .engine
        .take()
        .map(|engine| engine.join())
        .transpose()
        .map_err(|error| AppError(error.to_string()));
    result?;
    joined?;
    Ok(())
}
fn message_loop(window: &MainWindow, app: &mut AppState) -> Result<(), AppError> {
    let accelerators =
        unsafe { LoadAcceleratorsW(Some(platform::instance()?), resource(ACCELERATORS))? };
    let mut msg = MSG::default();
    loop {
        let value = unsafe { GetMessageW(&mut msg, None, 0, 0) }.0;
        if value == -1 {
            return Err(windows::core::Error::from_thread().into());
        }
        if value == 0 {
            return Ok(());
        }
        if !app.scheduler.is_dialog_message(&msg)
            && !app.details.is_dialog_message(&msg)
            && !app.completions.is_dialog_message(&msg)
        {
            let translated = unsafe { TranslateAcceleratorW(window.hwnd, accelerators, &msg) != 0 };
            if !translated && !unsafe { IsDialogMessageW(window.hwnd, &msg).as_bool() } {
                unsafe {
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
        }
        // End every Signals RefCell borrow before handlers call native APIs.
        loop {
            let action = { window.signals.actions.borrow_mut().pop_front() };
            let Some(action) = action else {
                break;
            };
            if !window.signals.alive.get() {
                break;
            }
            if let Err(error) = app.handle(window, action) {
                message(Some(window.hwnd), "Rusty", &error.to_string(), false);
            }
        }
        app.scheduler_actions(window)?;
        app.completion_actions();
        if window.signals.overflow.replace(false) {
            message(
                Some(window.hwnd),
                "Command queue full",
                "Too many commands are pending. Try again after the next update.",
                false,
            );
        }
    }
}
unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    std::panic::catch_unwind(|| unsafe { window_inner(hwnd, msg, wparam, lparam) })
        .unwrap_or(LRESULT(0))
}
unsafe fn window_inner(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        if msg == WM_NCCREATE {
            if lparam.0 == 0 {
                return LRESULT(0);
            }
            let create = &*(lparam.0 as *const CREATESTRUCTW);
            let pointer = create.lpCreateParams as *const Signals;
            if pointer.is_null() {
                return LRESULT(0);
            }
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, pointer as isize);
            (*pointer).alive.set(true);
            // Default nonclient creation stores the title passed to CreateWindowExW.
            // Returning TRUE ourselves skips that initialization and leaves it blank.
            return DefWindowProcW(hwnd, msg, wparam, lparam);
        }
        let pointer = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Signals;
        if !pointer.is_null() {
            let signals = &*pointer;
            match msg {
                WM_CLOSE => {
                    signals.push(Action::Close);
                    return LRESULT(0);
                }
                WM_QUERYENDSESSION => return LRESULT(1),
                WM_ENDSESSION if wparam.0 != 0 => {
                    // Atomic cancellation is independent of the outer message
                    // loop, including while a modal file-info dialog is open.
                    if let Some(client) = signals.session_shutdown.borrow().as_ref() {
                        client.request_shutdown();
                    }
                    signals.push(Action::EndSession);
                    return LRESULT(0);
                }
                WM_COMMAND => {
                    signals.push(Action::Command((wparam.0 & 0xffff) as u16));
                    return LRESULT(0);
                }
                WM_CONTEXTMENU if GetDlgCtrlID(HWND(wparam.0 as *mut _)) == TREE as i32 => {
                    signals.push(Action::QueueMenu(POINT {
                        x: (lparam.0 as u16 as i16) as i32,
                        y: ((lparam.0 >> 16) as u16 as i16) as i32,
                    }));
                    return LRESULT(0);
                }
                WM_CONTEXTMENU if GetDlgCtrlID(HWND(wparam.0 as *mut _)) == LIST as i32 => {
                    signals.push(Action::ContextMenu(POINT {
                        x: (lparam.0 as u16 as i16) as i32,
                        y: ((lparam.0 >> 16) as u16 as i16) as i32,
                    }));
                    return LRESULT(0);
                }
                WM_SIZE => {
                    if wparam.0 == SIZE_MINIMIZED as usize {
                        // Main-window ownership is handled outside the callback.
                        signals.push(Action::Minimize);
                    } else {
                        signals.push(Action::Layout);
                    }
                }
                tray::TRAY_CALLBACK => {
                    match (lparam.0 as u32) & 0xffff {
                        WM_CONTEXTMENU => signals.push(Action::TrayMenu),
                        windows::Win32::UI::Shell::NIN_SELECT
                        | tray::TRAY_KEY_SELECT
                        | WM_LBUTTONDBLCLK => signals.push(Action::Command(SHOW_MANAGER)),
                        _ => {}
                    }
                    return LRESULT(0);
                }
                msg if msg != 0 && msg == signals.taskbar_created.get() => {
                    signals.push(Action::RestoreTray);
                    return LRESULT(0);
                }
                WM_DPICHANGED if lparam.0 != 0 => {
                    signals.push(Action::Dpi(
                        (wparam.0 & 0xffff) as u32,
                        *(lparam.0 as *const RECT),
                    ));
                    return LRESULT(0);
                }
                WM_GETMINMAXINFO if lparam.0 != 0 => {
                    let info = &mut *(lparam.0 as *mut MINMAXINFO);
                    let dpi = GetDpiForWindow(hwnd).max(96);
                    info.ptMinTrackSize.x = scale(910, dpi);
                    info.ptMinTrackSize.y = scale(360, dpi);
                    return LRESULT(0);
                }
                WM_NOTIFY if lparam.0 != 0 => {
                    let header = &*(lparam.0 as *const NMHDR);
                    match (header.idFrom as u16, header.code) {
                        (TOOLBAR, TBN_GETINFOTIPW) => {
                            let info = &mut *(lparam.0 as *mut NMTBGETINFOTIPW);
                            if let Some(button) = crate::icons::BUTTONS
                                .iter()
                                .find(|button| button.command == info.iItem as u16)
                            {
                                if !info.pszText.is_null() && info.cchTextMax > 0 {
                                    let text = platform::wide(button.tooltip);
                                    let count = (text.len() - 1).min(info.cchTextMax as usize - 1);
                                    std::ptr::copy_nonoverlapping(
                                        text.as_ptr(),
                                        info.pszText.0,
                                        count,
                                    );
                                    *info.pszText.0.add(count) = 0;
                                }
                            }
                            return LRESULT(0);
                        }
                        (TOOLBAR, TBN_DROPDOWN) => {
                            let notification = &*(lparam.0 as *const NMTOOLBARW);
                            if matches!(notification.iItem as u16, QUEUE_START | QUEUE_STOP) {
                                signals.push(Action::ToolbarQueue(notification.iItem as u16));
                                return LRESULT(TBDDRET_DEFAULT as isize);
                            }
                        }

                        (TREE, TVN_SELCHANGEDW) => {
                            let tree = &*(lparam.0 as *const NMTREEVIEWW);
                            signals.push(Action::Category(tree.itemNew.lParam.0));
                        }
                        (LIST, LVN_ITEMCHANGED) => signals.push(Action::Selection),
                        (LIST, NM_DBLCLK) => {
                            let item = &*(lparam.0 as *const NMITEMACTIVATE);
                            if item.iItem >= 0 {
                                signals.push(Action::DoubleClick);
                            }
                        }
                        (LIST, NM_RETURN) => signals.push(Action::Command(DETAILS)),
                        _ => {}
                    }
                }
                crate::shell_icons::ASSOCIATIONS_CHANGED => {
                    signals.push(Action::AssociationsChanged);
                    return LRESULT(0);
                }
                WM_CLIPBOARDUPDATE => signals.push(Action::Clipboard),
                WM_SETTINGCHANGE => signals.push(Action::AssociationsChanged),
                bridge::BROWSER_WAKE => signals.push(Action::Browser),
                bridge::ENGINE_WAKE | WM_TIMER => {
                    signals.push(Action::Engine);
                    return LRESULT(0);
                }
                WM_DESTROY => {
                    PostQuitMessage(0);
                    return LRESULT(0);
                }
                WM_NCDESTROY => {
                    signals.alive.set(false);
                    SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                }
                _ => {}
            }
        }
        DefWindowProcW(hwnd, msg, wparam, lparam)
    }
}

impl AppState {
    fn remember_download(&mut self, owner: HWND, download: &dialogs::AddedDownload) {
        if download.remember_path {
            let mut updated = self.settings.clone();
            crate::download_form::remember_folder(
                &mut updated,
                download.request.category.as_deref().unwrap_or("Automatic"),
                &download.request.destination,
            );
            match settings::save(&updated) {
                Ok(()) => self.settings = updated,
                Err(e) => {
                    message(Some(owner), "Download folder", &e.to_string(), false);
                }
            }
        }
    }
    fn add_batch(&mut self, window: &MainWindow) -> Result<(), AppError> {
        let Some(text) = crate::options_editors::text(
            window.hwnd,
            "Add batch URLs",
            "One HTTP(S) URL per line. Accepted files are held in a queue.",
            "",
            65535,
        )?
        else {
            return Ok(());
        };
        let lines: Vec<_> = text
            .lines()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        if lines.len() > 128 {
            message(
                Some(window.hwnd),
                "Batch URLs",
                "Enter at most 128 URLs per batch.",
                false,
            );
            return Ok(());
        }
        let client = self.engine.as_ref().expect("running engine").client();
        let queue = if self.settings.configuration.choose_queue_batch {
            let Some(queue) = crate::queue_picker::choose(window.hwnd, &client)? else {
                return Ok(());
            };
            queue
        } else {
            rusty_engine::QueueId::MAIN
        };
        let mut results = vec![];
        for (n, address) in lines.into_iter().enumerate() {
            let filename = match crate::download_form::suggested_filename(address) {
                Ok(name) => name,
                Err(_) => {
                    results.push(format!("{}: invalid HTTP(S) URL", n + 1));
                    continue;
                }
            };
            let path = crate::download_form::suggested_path(&self.settings, &filename, "Automatic");
            let mut request = match rusty_engine::validate_request(
                address,
                path,
                rusty_engine::ReplacementPolicy::Deny,
            ) {
                Ok(request) => request,
                Err(_) => {
                    results.push(format!("{}: invalid address or filename", n + 1));
                    continue;
                }
            };
            request.options = self.settings.configuration.download_options(true);
            request.deferred = true;
            request.queue = Some(queue);
            let download = dialogs::AddedDownload {
                request,
                remember_path: false,
            };
            let intent = self.resolve_download_intent(window, download, None)?;
            let result = match intent {
                DownloadIntent::New(download) => {
                    let (lease, admission) = crate::provisional::manual(&client, &download)?;
                    match crate::provisional::wait_admission(window.hwnd, &lease, admission) {
                        Ok(Some(_)) => "held in queue",
                        Ok(None) => "cancelled",
                        Err(_) => "admission failed",
                    }
                }
                DownloadIntent::Existing { .. } | DownloadIntent::RestartApproved { .. } => {
                    "duplicate kept; no batch transfer started"
                }
                DownloadIntent::Cancelled => "cancelled",
            };
            results.push(format!("{}: {} — {}", n + 1, filename, result));
        }
        message(
            Some(window.hwnd),
            "Batch results",
            &results.join("\n"),
            false,
        );
        Ok(())
    }
    fn prepared_intent(
        &mut self,
        window: &MainWindow,
        prepared: dialogs::PreparedDownload,
        lease: Option<&CaptureLease>,
    ) -> Result<DownloadIntent, AppError> {
        let client = self.engine.as_ref().expect("running engine").client();
        let immediate = self.settings.configuration.show_start_dialog
            && self.settings.configuration.immediate_info;
        let download = if immediate {
            prepared.initial(window.hwnd, &self.settings, lease, &client)?
        } else {
            prepared.edit(window.hwnd, &self.settings, lease, &client)?
        };
        let Some(mut download) = download else {
            return Ok(DownloadIntent::Cancelled);
        };
        if immediate && !download.request.deferred {
            download.request.options.confirmation_required = true;
        }
        self.resolve_download_intent(window, download, lease)
    }
    fn execute_manual_intent(
        &mut self,
        owner: HWND,
        intent: DownloadIntent,
    ) -> Result<(), AppError> {
        match intent {
            DownloadIntent::New(download) => {
                if download.request.options.confirmation_required {
                    let client = self.engine.as_ref().expect("running engine").client();
                    let (lease, admission) = crate::provisional::manual(&client, &download)?;
                    crate::provisional::show(
                        owner,
                        &client,
                        &download,
                        &lease,
                        admission,
                        self.settings.configuration.choose_queue_later,
                    )?;
                    self.remember_download(owner, &download);
                } else if self.command(owner, Command::Submit(download.request.clone())) {
                    self.remember_download(owner, &download);
                }
            }
            DownloadIntent::Existing { job_id, action } => {
                match action {
                    ExistingAction::Restart => {
                        self.command(owner, Command::Restart(job_id));
                    }
                    ExistingAction::Activate => {
                        self.command(owner, Command::Activate(job_id));
                    }
                    ExistingAction::Show => {}
                }
                if action != ExistingAction::Restart {
                    if let Some(job) = self.view.jobs.get(&job_id) {
                        self.details.show_or_update(owner, job)?;
                    }
                }
            }
            DownloadIntent::RestartApproved {
                job_id,
                replacement,
            } => {
                let client = self.engine.as_ref().expect("running engine").client();
                let (lease, admission) = crate::provisional::manual_action(
                    &client,
                    rusty_engine::submissions::SubmissionAction::RestartApproved(
                        job_id,
                        replacement,
                    ),
                )?;
                crate::provisional::wait_admission(owner, &lease, admission)?;
            }
            DownloadIntent::Cancelled => {}
        }
        Ok(())
    }
    fn browser_events(&mut self, window: &MainWindow) -> Result<(), AppError> {
        let Some(browser) = self.browser.as_ref() else {
            return Ok(());
        };
        if browser.take_activation() {
            platform::bring_to_front(window.hwnd);
        }
        let presentations = browser.take_presentations();
        let captures = browser.drain();
        if !presentations.is_empty() {
            self.events(window);
            if self.lifecycle.accepts_submissions() {
                for id in presentations {
                    if let Some(snapshot) = self.view.jobs.get(&id) {
                        if self.settings.configuration.progress.show {
                            self.details.observe(window.hwnd, snapshot)?;
                        }
                    }
                }
            }
        }
        for incoming in captures {
            let id = incoming.capture.id.clone();
            if let Some(target) = self.refresh_target {
                let eligible = self.lifecycle.accepts_submissions()
                    && incoming.lease.state() == LeaseState::Pending
                    && self.view.jobs.get(&target).is_some_and(|job| job.refreshable && (matches!(job.state, rusty_engine::JobState::Paused | rusty_engine::JobState::Failed) || job.state == rusty_engine::JobState::Waiting && job.deferred));
                if !eligible {
                    self.browser.as_ref().expect("controller").decline(&id, DeclineCode::BrowserFallback);
                    self.refresh_target = None;
                    self.browser.as_ref().expect("controller").set_refresh_armed(false);
                    continue;
                }
                let job = self.view.jobs.get(&target).expect("checked");
                let display = url::Url::parse(&incoming.capture.url).ok().map(|mut url| {
                    url.set_query(None); url.set_fragment(None); url.into_string()
                }).unwrap_or_else(|| "the new address".into());
                if !message(Some(window.hwnd), "Use new download link?",
                    &format!("Continue {} using this browser download?\n\n{}\n\nRusty will verify the saved partial before taking over.", job.filename, display), true)
                {
                    self.browser.as_ref().expect("controller").decline(&id, DeclineCode::UserCancelled);
                    continue;
                }
                let candidate = RefreshCandidate {
                    url: incoming.capture.url.clone(),
                    source_page: incoming.capture.source_page.clone(),
                    request_context: incoming.capture.request_context.clone(),
                };
                let submitted = self.browser.as_ref().expect("controller").submit_refresh(
                    id.clone(), target, candidate, incoming.lease.clone(),
                );
                if let Err(error) = submitted {
                    self.browser.as_ref().expect("controller").decline(&id, if error == CommandError::QueueFull { DeclineCode::Busy } else { DeclineCode::AppClosing });
                } else {
                    self.refresh_target = None;
                    self.browser.as_ref().expect("controller").set_refresh_armed(false);
                }
                continue;
            }
            let intent = if self.lifecycle.accepts_submissions()
                && incoming.lease.state() == LeaseState::Pending
            {
                match dialogs::prepare_browser_download(
                    &self.settings,
                    &incoming.capture,
                    &incoming.lease,
                ) {
                    Ok(Some(prepared)) => {
                        self.prepared_intent(window, prepared, Some(&incoming.lease))
                    }
                    Ok(None) => Ok(DownloadIntent::Cancelled),
                    Err(e) => Err(e),
                }
            } else {
                Ok(DownloadIntent::Cancelled)
            };
            let intent = match intent {
                Ok(mut intent) => {
                    if let DownloadIntent::New(download) = &mut intent {
                        download.request.source_page = incoming.capture.source_page.clone();
                    }
                    intent
                },
                Err(_) => {
                    self.browser
                        .as_ref()
                        .expect("controller")
                        .decline(&id, DeclineCode::InvalidRequest);
                    continue;
                }
            };
            if let DownloadIntent::New(download) = &intent {
                self.remember_download(window.hwnd, download);
            }
            let provisional = match &intent {
                DownloadIntent::New(download) if download.request.options.confirmation_required => {
                    Some(download.clone())
                }
                _ => None,
            };
            if let Some(action) = intent.into_submission() {
                if incoming.lease.state() == LeaseState::Pending {
                    let receipt = if provisional.is_some() {
                        Some(
                            self.browser
                                .as_ref()
                                .expect("controller")
                                .subscribe_receipt(&id)
                                .map_err(|e| AppError(e.to_string()))?,
                        )
                    } else {
                        None
                    };
                    if let Err(e) = self.browser.as_ref().expect("controller").submit(
                        id.clone(),
                        action,
                        incoming.lease.clone(),
                        incoming.reply,
                    ) {
                        self.browser.as_ref().expect("controller").decline(
                            &id,
                            if e == CommandError::QueueFull {
                                DeclineCode::Busy
                            } else {
                                DeclineCode::AppClosing
                            },
                        );
                    } else if let (Some(download), Some(receipt)) = (provisional, receipt) {
                        let client = self.engine.as_ref().expect("running engine").client();
                        if let Err(error) = crate::provisional::show(
                            window.hwnd,
                            &client,
                            &download,
                            &incoming.lease,
                            crate::provisional::Admission::Browser(id.clone(), receipt),
                            self.settings.configuration.choose_queue_later,
                        ) {
                            message(
                                Some(window.hwnd),
                                "Download File Info",
                                &error.to_string(),
                                false,
                            );
                        }
                    }
                }
            } else {
                self.browser
                    .as_ref()
                    .expect("controller")
                    .decline(&id, DeclineCode::UserCancelled);
            }
        }
        Ok(())
    }
}

impl AppState {
    fn completion_actions(&mut self) {
        let actions = self.completions.take_actions();
        for action in actions {
            match action {
                CompletionAction::Open {
                    owner,
                    path,
                    action,
                } => {
                    if !matches!(action, completion::OpenAction::OpenFolder)
                        && !self.scanner.allow_open(owner, &path)
                    {
                        continue;
                    }
                    if let Err(error) = completion::open_completed(owner, &path, action) {
                        message(
                            Some(owner),
                            "Cannot open download",
                            &error.to_string(),
                            false,
                        );
                    }
                }
                CompletionAction::ShowAgain(show) => {
                    let mut updated = self.settings.clone();
                    updated.show_completion_dialog = show;
                    match settings::save(&updated) {
                        Ok(()) => self.settings = updated,
                        Err(error) => {
                            message(None, "Completion preference", &error.to_string(), false);
                        }
                    }
                    self.completions
                        .set_show_again(self.settings.show_completion_dialog);
                }
            }
        }
        self.completions.prune();
    }
}

impl AppState {
    fn new_queue(&mut self, window: &MainWindow) -> Result<(), AppError> {
        if self.scheduler.request_new() {
            return Ok(());
        }
        self.queue_name(window)
    }
    fn queue_name(&mut self, window: &MainWindow) -> Result<(), AppError> {
        if self.pending_create.is_some() {
            message(
                Some(window.hwnd),
                "Create queue",
                "Wait for the previous queue creation to finish.",
                false,
            );
            return Ok(());
        }
        let owner = self.scheduler.hwnd().unwrap_or(window.hwnd);
        if let Some(name) = crate::queue_ui::name(owner, "Create new queue", "")? {
            self.events(window);
            if self.lifecycle.accepts_submissions() {
                let request = crate::scheduler::request_id();
                if self.command(window.hwnd, Command::CreateQueueForEditor { request, name }) {
                    self.pending_create = Some(request);
                    self.scheduler.creating(true);
                }
            }
        }
        Ok(())
    }
    fn queue_action(
        &mut self,
        window: &MainWindow,
        id: rusty_engine::QueueId,
        command: u16,
    ) -> Result<(), AppError> {
        match command {
            QUEUE_EDIT | QUEUE_SCHEDULE => self.scheduler.open(
                window.hwnd,
                &self.view,
                id,
                if command == QUEUE_EDIT {
                    Tab::Files
                } else {
                    Tab::Schedule
                },
            )?,
            QUEUE_RENAME => {
                let name = self
                    .view
                    .queues
                    .get(&id)
                    .map(|q| q.name.clone())
                    .unwrap_or_default();
                if let Some(name) = crate::queue_ui::name(window.hwnd, "Rename queue", &name)? {
                    self.command(window.hwnd, Command::RenameQueue(id, name));
                }
            }
            QUEUE_DELETE => {
                if message(
                    Some(window.hwnd),
                    "Delete queue?",
                    "Delete this empty queue and its schedule?",
                    true,
                ) {
                    self.command(window.hwnd, Command::DeleteQueue(id));
                }
            }
            QUEUE_START => {
                if !self.scheduler.request_start(id) {
                    self.command(window.hwnd, Command::StartQueue(id));
                }
            }
            QUEUE_STOP => {
                self.scheduler.cancel_start(id);
                self.command(window.hwnd, Command::StopQueue(id));
            }
            QUEUE_ONE | QUEUE_TWO => {
                self.command(
                    window.hwnd,
                    Command::QueueParallelism(id, if command == QUEUE_ONE { 1 } else { 2 }),
                );
            }
            _ => {}
        }
        Ok(())
    }
    fn scheduler_actions(&mut self, window: &MainWindow) -> Result<(), AppError> {
        if !self.lifecycle.accepts_submissions() {
            self.scheduler.close_all();
            return Ok(());
        }
        for action in self.scheduler.poll(&self.view)? {
            match action {
                EditorAction::Command(command) => {
                    if !self.command(window.hwnd, command) {
                        self.scheduler.rejected();
                    }
                }
                EditorAction::NewQueue => self.queue_name(window)?,
                EditorAction::ExitApplication => {
                    self.close_after_editor(window.hwnd);
                    break;
                }
                EditorAction::Error(error) => {
                    message(
                        self.scheduler.hwnd().or(Some(window.hwnd)),
                        "Queue editor",
                        &error,
                        false,
                    );
                }
            }
        }
        Ok(())
    }
}
