use crate::{
    bridge::{self, WakeTarget},
    commands::*,
    controls::{scale, Controls},
    details::DetailsWindows,
    dialogs,
    duplicates::{self, DuplicateChoice},
    lifecycle::Lifecycle,
    platform::{self, message, resource, ComApartment},
    presenter::{Category, ViewModel},
    settings::{self, Settings},
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
use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    sync::Arc,
};
use windows::{
    core::{w, PCWSTR},
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM},
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
    Category(Category),
    Selection,
    Layout,
    Dpi(u32, RECT),
    Engine,
    Browser,
    Close,
}
#[derive(Default)]
struct Signals {
    actions: RefCell<VecDeque<Action>>,
    alive: Cell<bool>,
    overflow: Cell<bool>,
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
            )
        });
        if duplicate {
            return;
        }
        if actions.len() >= 128 && matches!(action, Action::Command(_)) {
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
                hIcon: LoadIconW(None, IDI_APPLICATION)?,
                hbrBackground: GetSysColorBrush(COLOR_WINDOW),
                ..Default::default()
            };
            if RegisterClassW(&class) == 0 {
                return Err(windows::core::Error::from_thread().into());
            }
            let signals = Box::<Signals>::default();
            let dpi = GetDpiForSystem().max(96);
            let hwnd = CreateWindowExW(
                WS_EX_CONTROLPARENT,
                class.lpszClassName,
                w!("Rusty Download Manager"),
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
    engine: Option<EngineHandle>,
    view: ViewModel,
    controls: Controls,
    details: DetailsWindows,
    settings: Settings,
    category: Category,
    lifecycle: Lifecycle,
    shutdown_sent: bool,
    recovery_reported: bool,
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
                    Err(error) => message(
                        Some(owner),
                        "Duplicate preference",
                        &format!(
                            "This choice will be used now, but could not be saved.\n\n{error}"
                        ),
                        false,
                    ),
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
                Ok(DownloadIntent::Existing {
                    job_id: existing.id,
                    action: ExistingAction::Restart,
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
                    let action = if latest.state == rusty_engine::JobState::Completed
                        && !latest.destination.is_file()
                    {
                        ExistingAction::Restart
                    } else if !latest.state.is_terminal() || latest.state.can_retry() {
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
        if self.lifecycle != Lifecycle::Open {
            return;
        }
        let active = self.view.active();
        let accepted = !active || message(Some(owner), "Stop downloads and exit?", "Stop all running and waiting downloads, clean up their partial files, and exit Rusty?", true);
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
        let changed = !events.is_empty();
        for event in events {
            match &event {
                Event::JobChanged(snapshot) => {
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
                }
                Event::JobRemoved(id) => self.details.remove(*id),
                Event::CommandRejected { message: text } => {
                    message(Some(window.hwnd), "Download command", text, false);
                }
                Event::ShutdownComplete => {
                    self.lifecycle.engine_stopped();
                }
            }
            self.view.apply(event);
        }
        for command in self.details.take_commands() {
            if self.lifecycle == Lifecycle::Open {
                self.command(window.hwnd, command);
            }
        }
        self.details.prune();
        if changed {
            self.controls.refresh(
                window.hwnd,
                &self.view,
                self.category,
                self.lifecycle != Lifecycle::Open,
            );
        }
        if self.lifecycle == Lifecycle::Closing {
            self.send_shutdown(window.hwnd);
        }
        if self.lifecycle == Lifecycle::Closed {
            if !self.recovery_reported {
                self.recovery_reported = true;
                if let Some(report) = self.view.recovery_notice() {
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
            Action::Browser => self.browser_events(window)?,
            Action::Close => self.close(owner),
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
            Action::Category(category) => {
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
            Action::Command(id) if self.lifecycle.accepts_submissions() => {
                let selected = self.controls.selected();
                let actions = self.view.actions(selected);
                match id {
                    ADD => {
                        if let Some(download) = dialogs::show_add_download(owner, &self.settings)? {
                            let intent = self.resolve_download_intent(window, download, None)?;
                            self.execute_manual_intent(owner, intent)?;
                        }
                    }
                    START if actions.start => {
                        if let Some(id) = selected {
                            let command = if self
                                .view
                                .jobs
                                .get(&id)
                                .is_some_and(|s| s.state == rusty_engine::JobState::Paused)
                            {
                                Command::Resume(id)
                            } else {
                                Command::Start(id)
                            };
                            self.command(owner, command);
                        }
                    }
                    OPTIONS => {
                        if let Some(settings) = dialogs::show_options(owner, &self.settings)? {
                            settings::save(&settings)?;
                            self.settings = settings;
                        }
                    }
                    STOP if actions.stop => {
                        if let Some(id) = selected {
                            self.command(owner, Command::Cancel(id));
                        }
                    }
                    STOP_ALL if actions.stop_all => {
                        self.command(owner, Command::CancelAll);
                    }
                    REMOVE if actions.remove => {
                        if let Some(id) = selected {
                            self.command(owner, Command::Remove(id));
                        }
                    }
                    RETRY if actions.retry => {
                        if let Some(id) = selected {
                            self.command(owner, Command::Retry(id));
                        }
                    }
                    DETAILS if actions.details => {
                        if let Some(snapshot) = selected.and_then(|id| self.view.jobs.get(&id)) {
                            self.details.show_or_update(owner, snapshot)?;
                        }
                    }
                    OPEN_FILE if actions.open_file => self.open(owner, selected, false)?,
                    OPEN_FOLDER if actions.open_folder => self.open(owner, selected, true)?,
                    CATEGORY_ALL => {
                        self.controls.select_all_category();
                        self.category = Category::All;
                        self.controls
                            .refresh(owner, &self.view, self.category, false);
                    }
                    ABOUT => {
                        message(Some(owner), "About Rusty", "Rusty Download Manager 0.1\n\nNative Windows controls and single-stream HTTP/HTTPS downloads.\nTwo simultaneous downloads. History lasts for this session.\n\nPause/resume, replacement, queues and browser capture are not available yet.", false);
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        Ok(())
    }
    fn open(
        &self,
        owner: HWND,
        selected: Option<rusty_engine::JobId>,
        folder: bool,
    ) -> Result<(), AppError> {
        let Some(job) = selected.and_then(|id| self.view.jobs.get(&id)) else {
            return Ok(());
        };
        let path = if folder {
            job.destination
                .parent()
                .ok_or_else(|| AppError("No containing folder is available.".into()))?
        } else {
            &job.destination
        };
        if if folder {
            !path.is_dir()
        } else {
            !path.is_file()
        } {
            return Err(AppError(
                "The selected file or folder no longer exists.".into(),
            ));
        }
        let path = platform::wide(path);
        let result = unsafe {
            windows::Win32::UI::Shell::ShellExecuteW(
                Some(owner),
                w!("open"),
                PCWSTR(path.as_ptr()),
                None,
                None,
                SW_SHOWNORMAL,
            )
        };
        if result.0 as isize <= 32 {
            return Err(AppError(
                "Windows could not open the selected file or folder.".into(),
            ));
        }
        Ok(())
    }
}

pub fn run() -> Result<(), AppError> {
    let session = UserSession::current()
        .map_err(|_| AppError("Cannot determine browser session identity.".into()))?;
    let _instance = match AppInstance::acquire_or_activate(&session)? {
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
                | ICC_STANDARD_CLASSES,
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
    let browser = BrowserController::new(engine.client(), Arc::new(move || wake.wake()));
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
        engine: Some(engine),
        view: ViewModel::default(),
        controls,
        details: DetailsWindows::default(),
        settings,
        category: Category::All,
        lifecycle: Lifecycle::Open,
        shutdown_sent: false,
        recovery_reported: false,
    };
    app.controls.layout(window.hwnd);
    app.controls
        .refresh(window.hwnd, &app.view, app.category, false);
    unsafe {
        let _ = ShowWindow(window.hwnd, SW_SHOW);
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
        if !app.details.is_dialog_message(&msg) {
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
            return LRESULT(1);
        }
        let pointer = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Signals;
        if !pointer.is_null() {
            let signals = &*pointer;
            match msg {
                WM_CLOSE => {
                    signals.push(Action::Close);
                    return LRESULT(0);
                }
                WM_QUERYENDSESSION => {
                    signals.push(Action::Close);
                    return LRESULT(0);
                }
                WM_COMMAND => {
                    signals.push(Action::Command((wparam.0 & 0xffff) as u16));
                    return LRESULT(0);
                }
                WM_SIZE => signals.push(Action::Layout),
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
                    info.ptMinTrackSize.x = scale(610, dpi);
                    info.ptMinTrackSize.y = scale(360, dpi);
                    return LRESULT(0);
                }
                WM_NOTIFY if lparam.0 != 0 => {
                    let header = &*(lparam.0 as *const NMHDR);
                    match (header.idFrom as u16, header.code) {
                        (TREE, TVN_SELCHANGEDW) => {
                            let tree = &*(lparam.0 as *const NMTREEVIEWW);
                            if let Some(category) =
                                Category::ALL.get(tree.itemNew.lParam.0 as usize)
                            {
                                signals.push(Action::Category(*category));
                            }
                        }
                        (LIST, LVN_ITEMCHANGED) => signals.push(Action::Selection),
                        (LIST, NM_DBLCLK | NM_RETURN) => signals.push(Action::Command(DETAILS)),
                        _ => {}
                    }
                }
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
                download.request.category.as_deref().unwrap_or("General"),
                &download.request.destination,
            );
            match settings::save(&updated) {
                Ok(()) => self.settings = updated,
                Err(e) => message(Some(owner), "Download folder", &e.to_string(), false),
            }
        }
    }
    fn execute_manual_intent(
        &mut self,
        owner: HWND,
        intent: DownloadIntent,
    ) -> Result<(), AppError> {
        match intent {
            DownloadIntent::New(download) => {
                if self.command(owner, Command::Submit(download.request.clone())) {
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
                        self.details.show_existing(owner, job)?;
                    }
                }
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
            unsafe {
                let _ = ShowWindow(window.hwnd, SW_RESTORE);
                let _ = SetForegroundWindow(window.hwnd);
            }
        }
        let captures = browser.drain();
        for incoming in captures {
            let id = incoming.capture.id.clone();
            let intent = if self.lifecycle.accepts_submissions()
                && incoming.lease.state() == LeaseState::Pending
            {
                match dialogs::show_browser_download(
                    window.hwnd,
                    &self.settings,
                    &incoming.capture,
                    &incoming.lease,
                ) {
                    Ok(Some(download)) => {
                        self.resolve_download_intent(window, download, Some(&incoming.lease))
                    }
                    Ok(None) => Ok(DownloadIntent::Cancelled),
                    Err(e) => Err(e),
                }
            } else {
                Ok(DownloadIntent::Cancelled)
            };
            let intent = match intent {
                Ok(intent) => intent,
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
            if let Some(action) = intent.into_submission() {
                if incoming.lease.state() == LeaseState::Pending {
                    if let Err(e) = self.browser.as_ref().expect("controller").submit(
                        id.clone(),
                        action,
                        incoming.lease,
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
