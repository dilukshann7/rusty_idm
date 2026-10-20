//! Two bounded scanner workers. Scan failures never change transfer completion.
use crate::{platform::message, preferences::ScannerPreferences};
use rusty_engine::{JobId, JobSnapshot};
use std::{
    collections::{BTreeMap, VecDeque},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    time::{Duration, Instant},
};
use windows::Win32::Foundation::HWND;
struct Work {
    snapshot: JobSnapshot,
    settings: ScannerPreferences,
    token: u64,
    cancel: Arc<AtomicBool>,
}
struct Running {
    cancel: Arc<AtomicBool>,
    thread: std::thread::JoinHandle<()>,
}
struct Record {
    path: PathBuf,
    token: u64,
    result: Option<Result<(), String>>,
    cancel: Arc<AtomicBool>,
}
pub struct Scanner {
    next: u64,
    waiting: VecDeque<Work>,
    running: Vec<Running>,
    records: BTreeMap<JobId, Record>,
    tx: mpsc::Sender<(JobSnapshot, u64, Result<(), String>)>,
    rx: mpsc::Receiver<(JobSnapshot, u64, Result<(), String>)>,
}
impl Default for Scanner {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            next: 0,
            waiting: VecDeque::new(),
            running: vec![],
            records: BTreeMap::new(),
            tx,
            rx,
        }
    }
}
impl Scanner {
    pub fn invalidate(&mut self, id: JobId) {
        if let Some(record) = self.records.remove(&id) {
            record.cancel.store(true, Ordering::Release);
        }
        self.waiting.retain(|w| w.snapshot.id != id);
    }
    pub fn enqueue(&mut self, snapshot: JobSnapshot, settings: ScannerPreferences) {
        self.invalidate(snapshot.id);
        self.next = self.next.wrapping_add(1);
        let token = self.next;
        let cancel = Arc::new(AtomicBool::new(false));
        self.records.insert(
            snapshot.id,
            Record {
                path: snapshot.destination.clone(),
                token,
                result: None,
                cancel: cancel.clone(),
            },
        );
        if self.waiting.len() >= 128 {
            let _ = self.tx.send((
                snapshot,
                token,
                Err("Scanner queue is full; the completed file was not checked.".into()),
            ));
            return;
        }
        self.waiting.push_back(Work {
            snapshot,
            settings,
            token,
            cancel,
        });
        self.start();
    }
    fn start(&mut self) {
        let mut active = vec![];
        for worker in self.running.drain(..) {
            if worker.thread.is_finished() {
                let _ = worker.thread.join();
            } else {
                active.push(worker);
            }
        }
        self.running = active;
        while self.running.len() < 2 {
            let Some(work) = self.waiting.pop_front() else {
                break;
            };
            let tx = self.tx.clone();
            let cancel = work.cancel.clone();
            let fallback = (work.snapshot.clone(), work.token);
            match std::thread::Builder::new()
                .name("rusty-virus-check".into())
                .spawn(move || {
                    let result = run(&work);
                    let _ = tx.send((work.snapshot, work.token, result));
                }) {
                Ok(thread) => self.running.push(Running { cancel, thread }),
                Err(_) => {
                    let _ = self.tx.send((
                        fallback.0,
                        fallback.1,
                        Err("Cannot start the scanner worker.".into()),
                    ));
                }
            }
        }
    }
    pub fn drain(&mut self) -> Vec<(JobSnapshot, Result<(), String>)> {
        self.start();
        let mut ready = vec![];
        while let Ok((snapshot, token, result)) = self.rx.try_recv() {
            if let Some(record) = self.records.get_mut(&snapshot.id) {
                if record.token == token {
                    record.result = Some(result.clone());
                    ready.push((snapshot, result));
                }
            }
        }
        ready
    }
    pub fn allow_open(&self, owner: HWND, path: &Path) -> bool {
        let record = self.records.values().find(|r| r.path == path);
        match record.map(|r| &r.result) {
            Some(None) => {
                message(Some(owner),"Virus check pending","The file is still being checked. Wait for the scanner result before opening it.",false);
                false
            }
            Some(Some(Err(error))) => message(
                Some(owner),
                "Virus check did not succeed",
                &format!("{error}\n\nThe completed file is preserved. Open it anyway?"),
                true,
            ),
            _ => true,
        }
    }
}
impl Drop for Scanner {
    fn drop(&mut self) {
        for worker in &self.running {
            worker.cancel.store(true, Ordering::Release);
        }
        for worker in self.running.drain(..) {
            let _ = worker.thread.join();
        }
    }
}
fn run(work: &Work) -> Result<(), String> {
    work.settings.validate()?;
    if work.cancel.load(Ordering::Acquire) {
        return Err("Virus check cancelled; file was not verified.".into());
    }
    use std::os::windows::process::CommandExt;
    let mut command = std::process::Command::new(&work.settings.executable);
    for arg in &work.settings.arguments {
        if arg == "{file}" {
            command.arg(&work.snapshot.destination);
        } else {
            command.arg(arg);
        }
    }
    let mut child = command
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .creation_flags(0x08000000)
        .spawn()
        .map_err(|_| {
            "Cannot launch the configured scanner. The file was not checked.".to_owned()
        })?;
    let deadline = Instant::now() + Duration::from_secs(600);
    loop {
        if work.cancel.load(Ordering::Acquire) || Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err("Virus check cancelled or timed out. The file was not verified.".into());
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                return if status.success() {
                    Ok(())
                } else {
                    Err(format!(
                        "The scanner reported failure (exit code {}).",
                        status
                            .code()
                            .map(|v| v.to_string())
                            .unwrap_or_else(|| "unavailable".into())
                    ))
                }
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(100)),
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("Cannot read the scanner result. The file was not verified.".into());
            }
        }
    }
}
