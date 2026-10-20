//! Queue editor state is separate from engine scheduling and native callbacks.
use crate::{
    commands::*,
    platform::{self, message, wide},
    presenter::ViewModel,
    AppError,
};
use rusty_engine::{Command, JobId, QueueId, QueueSettings};
use std::sync::atomic::{AtomicU64, Ordering};
use windows::{
    core::PCWSTR,
    Win32::{Foundation::HWND, UI::WindowsAndMessaging::*},
};
#[path = "scheduler_form.rs"]
mod form;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Schedule,
    Files,
}
#[derive(Clone, Copy)]
enum Intent {
    Select(QueueId, Tab),
    New,
    Delete,
    Start,
    Close,
}
pub(super) enum UiAction {
    Button(i32, Option<JobId>),
    Changed,
    Select(QueueId),
    Tab(Tab),
    Dpi(u32),
    Open(QueueId, Tab),
    New,
}
pub enum EditorAction {
    Command(Command),
    NewQueue,
    Error(String),
}
struct PendingApply {
    request: u64,
    before: u64,
    accepted: bool,
    after: Option<Intent>,
}
struct Editor {
    form: form::Form,
    selected: QueueId,
    baseline: QueueSettings,
    baseline_revision: u64,
    dirty: bool,
    pending: Option<PendingApply>,
    tab: Tab,
    queued: Vec<UiAction>,
}
#[derive(Default)]
pub struct SchedulerWindow {
    editor: Option<Editor>,
}
pub fn request_id() -> u64 {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}
impl SchedulerWindow {
    pub fn is_open(&self) -> bool {
        self.editor.as_ref().is_some_and(|e| e.form.alive())
    }
    pub fn hwnd(&self) -> Option<HWND> {
        self.editor
            .as_ref()
            .filter(|e| e.form.alive())
            .map(|e| e.form.hwnd())
    }
    pub fn open(
        &mut self,
        owner: HWND,
        view: &ViewModel,
        id: QueueId,
        tab: Tab,
    ) -> Result<(), AppError> {
        if self.is_open() {
            let editor = self.editor.as_mut().expect("open editor");
            editor.queued.push(UiAction::Open(id, tab));
            editor.form.show();
            return Ok(());
        }
        let queue = view
            .queues
            .get(&id)
            .ok_or_else(|| AppError("This queue no longer exists.".into()))?;
        let mut form = form::Form::new(owner)?;
        let baseline = settings(queue);
        form.write(&baseline);
        form.page(tab);
        form.draw(view, id, false, false)?;
        form.show();
        self.editor = Some(Editor {
            form,
            selected: id,
            baseline,
            baseline_revision: queue.revision,
            dirty: false,
            pending: None,
            tab,
            queued: vec![],
        });
        Ok(())
    }
    pub fn request_new(&mut self) -> bool {
        if let Some(editor) = self.editor.as_mut().filter(|e| e.form.alive()) {
            editor.queued.push(UiAction::New);
            true
        } else {
            false
        }
    }
    pub fn result(&mut self, request: u64, result: &Result<(), String>) {
        if let Some(editor) = &mut self.editor {
            if editor
                .pending
                .as_ref()
                .is_some_and(|p| p.request == request)
            {
                match result {
                    Ok(()) => editor.pending.as_mut().expect("pending").accepted = true,
                    Err(_) => editor.pending = None,
                }
            }
        }
    }
    pub fn rejected(&mut self) {
        if let Some(editor) = &mut self.editor {
            editor.pending = None;
        }
    }
    pub fn is_dialog_message(&self, msg: &MSG) -> bool {
        self.editor
            .as_ref()
            .is_some_and(|e| e.form.is_dialog_message(msg))
    }
    pub fn poll(&mut self, view: &ViewModel) -> Result<Vec<EditorAction>, AppError> {
        let Some(editor) = self.editor.as_mut() else {
            return Ok(vec![]);
        };
        if !editor.form.alive() {
            self.editor = None;
            return Ok(vec![]);
        }
        let mut output = vec![];
        let mut close = false;
        let mut actions = editor.form.take_actions();
        actions.extend(std::mem::take(&mut editor.queued));
        for action in actions {
            match action {
                UiAction::Changed => {
                    if editor.pending.is_none() {
                        editor.dirty = true;
                    }
                }
                UiAction::Tab(tab) => {
                    editor.tab = tab;
                    editor.form.page(tab);
                }
                UiAction::Dpi(dpi) => editor.form.dpi(dpi)?,
                UiAction::Open(id, tab) => {
                    close |= editor.intent(Intent::Select(id, tab), view, &mut output)
                }
                UiAction::Select(id) => {
                    if id != editor.selected {
                        close |= editor.intent(Intent::Select(id, editor.tab), view, &mut output);
                    }
                }
                UiAction::New => close |= editor.intent(Intent::New, view, &mut output),
                UiAction::Button(id, file) => match id {
                    SCHED_APPLY => editor.apply(None, view, &mut output),
                    SCHED_CLOSE => close |= editor.intent(Intent::Close, view, &mut output),
                    SCHED_NEW => close |= editor.intent(Intent::New, view, &mut output),
                    SCHED_DELETE => close |= editor.intent(Intent::Delete, view, &mut output),
                    SCHED_START => close |= editor.intent(Intent::Start, view, &mut output),
                    SCHED_STOP => {
                        if let Some(pending) = &mut editor.pending {
                            if matches!(pending.after, Some(Intent::Start)) {
                                pending.after = None;
                            }
                        }
                        output.push(EditorAction::Command(Command::StopQueue(editor.selected)));
                    }
                    SCHED_UP | SCHED_DOWN => {
                        if let Some(file) = file {
                            output.push(EditorAction::Command(Command::MoveQueueEntry(
                                file,
                                id == SCHED_UP,
                            )));
                        }
                    }
                    SCHED_REMOVE => {
                        if let Some(file) = file {
                            output.push(EditorAction::Command(Command::AssignQueue(file, None)));
                        }
                    }
                    _ => {}
                },
            }
            if close {
                break;
            }
        }
        if !close {
            if let Some(queue) = view.queues.get(&editor.selected) {
                if editor
                    .pending
                    .as_ref()
                    .is_some_and(|p| p.accepted && queue.revision > p.before)
                {
                    let after = editor.pending.take().expect("pending").after;
                    editor.load(editor.selected, view);
                    if let Some(after) = after {
                        close |= editor.perform(after, view, &mut output);
                    }
                } else if !editor.dirty
                    && editor.pending.is_none()
                    && (editor.baseline != settings(queue)
                        || editor.baseline_revision != queue.revision)
                {
                    editor.load(editor.selected, view);
                }
            } else {
                if editor.dirty || editor.pending.is_some() {
                    output.push(EditorAction::Error("The selected queue was removed. Its unsaved editor settings could not be applied.".into()));
                }
                editor.pending = None;
                editor.dirty = false;
                if view.queues.contains_key(&QueueId::MAIN) {
                    editor.load(QueueId::MAIN, view);
                } else {
                    close = true;
                }
            }
        }
        if close {
            self.editor = None;
        } else {
            let editor = self.editor.as_mut().expect("editor");
            editor.form.draw(
                view,
                editor.selected,
                editor.dirty,
                editor.pending.is_some(),
            )?;
        }
        Ok(output)
    }
    pub fn close_all(&mut self) {
        self.editor = None;
    }
}
impl Editor {
    fn load(&mut self, id: QueueId, view: &ViewModel) {
        if let Some(queue) = view.queues.get(&id) {
            self.selected = id;
            self.baseline = settings(queue);
            self.baseline_revision = queue.revision;
            self.dirty = false;
            self.form.write(&self.baseline);
        }
    }
    fn intent(&mut self, intent: Intent, view: &ViewModel, output: &mut Vec<EditorAction>) -> bool {
        if self.pending.is_some() {
            output.push(EditorAction::Error(
                "Wait for the queue settings to finish saving.".into(),
            ));
            return false;
        }
        if self.dirty {
            let title = wide("Unsaved queue settings");
            let message=wide("Apply your changes first?\n\nYes: apply changes.\nNo: discard changes.\nCancel: keep editing.\n\nFile ordering and membership changes are already applied.");
            match unsafe {
                MessageBoxW(
                    Some(self.form.hwnd()),
                    PCWSTR(message.as_ptr()),
                    PCWSTR(title.as_ptr()),
                    MB_YESNOCANCEL | MB_ICONQUESTION | MB_DEFBUTTON3,
                )
            } {
                IDYES => {
                    self.apply(Some(intent), view, output);
                    return false;
                }
                IDNO => self.load(self.selected, view),
                _ => return false,
            }
        }
        self.perform(intent, view, output)
    }
    fn apply(&mut self, after: Option<Intent>, view: &ViewModel, output: &mut Vec<EditorAction>) {
        if self.pending.is_some() {
            return;
        }
        let Some(queue) = view.queues.get(&self.selected) else {
            output.push(EditorAction::Error("This queue no longer exists.".into()));
            return;
        };
        let settings = match self.form.read() {
            Ok(settings) => settings,
            Err(error) => {
                output.push(EditorAction::Error(error));
                return;
            }
        };
        if let Some(now) = rusty_engine::LocalStamp::now() {
            if let Err(error) = settings.schedule.validate_for_apply(now) {
                output.push(EditorAction::Error(error));
                return;
            }
        }
        let request = request_id();
        self.pending = Some(PendingApply {
            request,
            before: queue.revision,
            accepted: false,
            after,
        });
        output.push(EditorAction::Command(Command::ConfigureQueue {
            request,
            id: self.selected,
            settings,
        }));
    }
    fn perform(
        &mut self,
        intent: Intent,
        view: &ViewModel,
        output: &mut Vec<EditorAction>,
    ) -> bool {
        match intent {
            Intent::Select(id, tab) => {
                if view.queues.contains_key(&id) {
                    self.load(id, view);
                    self.tab = tab;
                    self.form.page(tab);
                }
            }
            Intent::New => output.push(EditorAction::NewQueue),
            Intent::Delete => {
                if view
                    .queues
                    .get(&self.selected)
                    .is_some_and(|q| q.id != QueueId::MAIN && q.entries.is_empty())
                    && message(
                        Some(self.form.hwnd()),
                        "Delete queue?",
                        "Delete this empty queue and its schedule?",
                        true,
                    )
                {
                    output.push(EditorAction::Command(Command::DeleteQueue(self.selected)));
                }
            }
            Intent::Start => output.push(EditorAction::Command(Command::StartQueue(self.selected))),
            Intent::Close => return true,
        }
        false
    }
}
fn settings(queue: &rusty_engine::QueueSnapshot) -> QueueSettings {
    QueueSettings {
        parallelism: queue.parallelism,
        schedule: queue.schedule.clone(),
    }
}
