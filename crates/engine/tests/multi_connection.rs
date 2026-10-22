#[allow(dead_code)]
mod support;

use rusty_engine::{
    spawn_engine_in_memory, validate_request, Command, Event, JobId, JobSnapshot, JobState,
    ReplacementPolicy,
};
use std::{
    io::Write,
    net::{TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};

const SIZE: u64 = 16 * 1024 * 1024;

#[derive(Clone, Copy)]
enum Mode {
    Exact,
    Malformed,
    IgnoreRanges,
    ChangedValidator,
    Dynamic,
    LastModified,
    TransientProbe,
    InterruptedRanges,
    ChangedAfterDrop,
    IgnoredAfterDrop,
    RejectedProbe,
}

fn byte_at(offset: u64) -> u8 {
    ((offset * 31 + offset / 251) % 256) as u8
}

#[derive(Default)]
struct Counts {
    probes: AtomicUsize,
    dropped: AtomicBool,
    workers: AtomicUsize,
    peak: AtomicUsize,
    ranged_workers: AtomicUsize,
}

fn respond(mut stream: TcpStream, counts: Arc<Counts>, size: u64, mode: Mode) {
    let request = support::request(&mut stream);
    let range = request.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        if !name.eq_ignore_ascii_case("range") {
            return None;
        }
        let (first, last) = value.trim().strip_prefix("bytes=")?.split_once('-')?;
        Some((first.parse::<u64>().ok()?, last.parse::<u64>().ok()?))
    });
    let (start, end) = if matches!(mode, Mode::IgnoreRanges) {
        (0, size - 1)
    } else if let Some(pair) = range {
        pair
    } else if matches!(mode, Mode::RejectedProbe) {
        (0, size - 1)
    } else {
        return;
    };
    if start > end || end >= size {
        return;
    }
    let is_probe = range == Some((0, 0));
    if is_probe {
        let previous = counts.probes.fetch_add(1, Ordering::SeqCst);
        if matches!(mode, Mode::RejectedProbe) {
            let _ = stream.write_all(b"HTTP/1.1 416 Range Not Satisfiable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
            return;
        }
        if matches!(mode, Mode::TransientProbe) && previous == 0 {
            let _ = stream.write_all(b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
            return;
        }
        if matches!(mode, Mode::IgnoredAfterDrop) && previous > 0 {
            let _ = stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
            return;
        }
    }
    if !is_probe {
        let active = counts.workers.fetch_add(1, Ordering::SeqCst) + 1;
        counts.peak.fetch_max(active, Ordering::SeqCst);
        if start > 0 {
            counts.ranged_workers.fetch_add(1, Ordering::SeqCst);
        }
    }
    let validator = if (matches!(mode, Mode::ChangedValidator) && !is_probe)
        || (matches!(mode, Mode::ChangedAfterDrop) && counts.probes.load(Ordering::SeqCst) > 1)
    {
        "\"large-v2\""
    } else {
        "\"large-v1\""
    };
    let validator_header = if matches!(mode, Mode::LastModified) {
        "Last-Modified: Wed, 21 Oct 2026 00:00:00 GMT".to_owned()
    } else {
        format!("ETag: {validator}")
    };
    if matches!(mode, Mode::IgnoreRanges | Mode::RejectedProbe) {
        let _ = write!(stream, "HTTP/1.1 200 OK\r\nContent-Length: {size}\r\n{validator_header}\r\nConnection: close\r\n\r\n");
    } else {
        let declared_start = if matches!(mode, Mode::Malformed) && start > 0 {
            start + 1
        } else {
            start
        };
        let _ = write!(stream,
            "HTTP/1.1 206 Partial Content\r\nContent-Range: bytes {declared_start}-{end}/{size}\r\nContent-Length: {}\r\n{validator_header}\r\nConnection: close\r\n\r\n",
            end - start + 1,
        );
    }
    let mut offset = start;
    let interrupt = !is_probe
        && matches!(
            mode,
            Mode::InterruptedRanges | Mode::ChangedAfterDrop | Mode::IgnoredAfterDrop
        )
        && !counts.dropped.swap(true, Ordering::SeqCst);
    let mut chunk = [0u8; 32 * 1024];
    while offset <= end {
        let len = (end - offset + 1).min(chunk.len() as u64) as usize;
        for (index, byte) in chunk[..len].iter_mut().enumerate() {
            *byte = byte_at(offset + index as u64);
        }
        if stream.write_all(&chunk[..len]).is_err() {
            break;
        }
        offset += len as u64;
        if interrupt && offset - start >= 256 * 1024 {
            break;
        }
        if matches!(mode, Mode::Dynamic) && start == 0 && !is_probe {
            thread::sleep(Duration::from_millis(4));
        } else if !is_probe && !matches!(mode, Mode::Dynamic) {
            thread::sleep(Duration::from_millis(1));
        }
    }
    if !is_probe {
        counts.workers.fetch_sub(1, Ordering::SeqCst);
    }
}

struct Server {
    url: String,
    counts: Arc<Counts>,
    stop: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}
impl Server {
    fn new(size: u64, mode: Mode) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/file", listener.local_addr().unwrap());
        listener.set_nonblocking(true).unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let counts = Arc::new(Counts::default());
        let stop_worker = stop.clone();
        let counts_worker = counts.clone();
        let thread = thread::spawn(move || {
            while !stop_worker.load(Ordering::Acquire) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        stream.set_nonblocking(false).unwrap();
                        let counts = counts_worker.clone();
                        thread::spawn(move || respond(stream, counts, size, mode));
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(2));
                    }
                    Err(_) => break,
                }
            }
        });
        Self {
            url,
            counts,
            stop,
            thread: Some(thread),
        }
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        self.thread.take().unwrap().join().unwrap();
    }
}

fn download(server: &Server, target: std::path::PathBuf, max_connections: u8) -> JobSnapshot {
    let mut request =
        validate_request(&server.url, target.clone(), ReplacementPolicy::Deny).unwrap();
    request.options.max_connections = max_connections;
    request.options.retry_policy.delay_seconds = 1;
    request.options.temporary_directory = Some(target.parent().unwrap().to_path_buf());
    let engine = spawn_engine_in_memory(Arc::new(|| {}));
    engine.try_send(Command::Submit(request)).unwrap();
    let deadline = Instant::now() + Duration::from_secs(20);
    let outcome = loop {
        let mut result = None;
        for event in engine.drain_events() {
            if let Event::JobChanged(job) = event {
                if job.state.is_terminal() {
                    result = Some(job);
                }
            }
        }
        if let Some(job) = result {
            break job;
        }
        assert!(Instant::now() < deadline, "range transfer did not finish");
        thread::sleep(Duration::from_millis(10));
    };
    engine.try_send(Command::Shutdown).unwrap();
    engine.join().unwrap();
    outcome
}

#[test]
fn downloads_one_file_through_concurrent_ranges() {
    let server = Server::new(SIZE, Mode::Exact);
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("large.bin");
    let completed = download(&server, target.clone(), 4);
    assert_eq!(
        completed.state,
        JobState::Completed,
        "{:?}",
        completed.error
    );
    assert_eq!(completed.received, SIZE);
    let bytes = std::fs::read(&target).unwrap();
    assert_eq!(bytes.len(), SIZE as usize);
    assert!(bytes
        .iter()
        .enumerate()
        .all(|(index, byte)| *byte == byte_at(index as u64)));
    assert!(server.counts.peak.load(Ordering::SeqCst) >= 2);
    assert!(server.counts.ranged_workers.load(Ordering::SeqCst) >= 1);
}

#[test]
fn transient_probe_failure_retries_instead_of_falling_back_or_becoming_restart_required() {
    let server = Server::new(SIZE, Mode::TransientProbe);
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("probe-retry.bin");
    let done = download(&server, target.clone(), 4);
    assert_eq!(done.state, JobState::Completed, "{:?}", done.error);
    assert!(server.counts.probes.load(Ordering::SeqCst) >= 2);
    assert!(server.counts.ranged_workers.load(Ordering::SeqCst) > 0);
    assert_eq!(
        std::fs::read(target).unwrap(),
        (0..SIZE).map(byte_at).collect::<Vec<_>>()
    );
}

#[test]
fn interrupted_ranges_resume_checkpoints_and_publish_exact_bytes() {
    let server = Server::new(SIZE, Mode::InterruptedRanges);
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("range-retry.bin");
    let done = download(&server, target.clone(), 4);
    assert_eq!(done.state, JobState::Completed, "{:?}", done.error);
    assert!(server.counts.dropped.load(Ordering::SeqCst));
    assert!(server.counts.probes.load(Ordering::SeqCst) >= 2);
    assert_eq!(
        std::fs::read(target).unwrap(),
        (0..SIZE).map(byte_at).collect::<Vec<_>>()
    );
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[test]
fn recovery_stops_when_ranges_are_ignored_or_the_validator_changes() {
    for mode in [Mode::ChangedAfterDrop, Mode::IgnoredAfterDrop] {
        let server = Server::new(SIZE, mode);
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("unsafe-retry.bin");
        let failed = download(&server, target.clone(), 4);
        assert_eq!(failed.state, JobState::Failed);
        assert_eq!(
            failed.error.unwrap().kind,
            rusty_engine::ErrorKind::RestartRequired
        );
        assert_eq!(server.counts.probes.load(Ordering::SeqCst), 2);
        assert!(!target.exists());
        assert!(std::fs::read_dir(dir.path()).unwrap().next().is_some());
    }
}

#[test]
fn malformed_worker_range_preserves_partial_without_publishing() {
    let server = Server::new(SIZE, Mode::Malformed);
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("bad.bin");
    let failed = download(&server, target.clone(), 4);
    assert_eq!(failed.state, JobState::Failed);
    assert!(!target.exists());
    assert!(std::fs::read_dir(dir.path()).unwrap().next().is_some());
}

#[test]
fn changed_validator_preserves_ranges_without_publishing() {
    let server = Server::new(SIZE, Mode::ChangedValidator);
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("changed.bin");
    let failed = download(&server, target.clone(), 4);
    assert_eq!(failed.state, JobState::Failed);
    assert!(!target.exists());
    assert!(std::fs::read_dir(dir.path()).unwrap().next().is_some());
}

#[test]
fn ignored_ranges_use_the_single_stream_fallback() {
    let server = Server::new(SIZE, Mode::IgnoreRanges);
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("fallback.bin");
    let done = download(&server, target.clone(), 4);
    assert_eq!(done.state, JobState::Completed, "{:?}", done.error);
    let bytes = std::fs::read(target).unwrap();
    assert_eq!(bytes.len(), SIZE as usize);
    assert!(bytes
        .iter()
        .enumerate()
        .all(|(index, byte)| *byte == byte_at(index as u64)));
    assert_eq!(server.counts.ranged_workers.load(Ordering::SeqCst), 0);
}

#[test]
fn an_initial_rejected_probe_still_falls_back_to_a_full_request() {
    let server = Server::new(SIZE, Mode::RejectedProbe);
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("rejected-probe.bin");
    let done = download(&server, target.clone(), 4);
    assert_eq!(done.state, JobState::Completed, "{:?}", done.error);
    assert_eq!(server.counts.probes.load(Ordering::SeqCst), 1);
    assert_eq!(
        std::fs::read(target).unwrap(),
        (0..SIZE).map(byte_at).collect::<Vec<_>>()
    );
}

#[test]
fn idle_connection_splits_a_slow_active_remainder() {
    let size = 32 * 1024 * 1024;
    let server = Server::new(size, Mode::Dynamic);
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("split.bin");
    let done = download(&server, target.clone(), 2);
    assert_eq!(done.state, JobState::Completed, "{:?}", done.error);
    assert_eq!(done.received, size);
    let bytes = std::fs::read(target).unwrap();
    assert_eq!(bytes.len(), size as usize);
    assert!(bytes
        .iter()
        .enumerate()
        .all(|(index, byte)| *byte == byte_at(index as u64)));
    assert!(server.counts.ranged_workers.load(Ordering::SeqCst) >= 2);
}

#[test]
fn paused_ranges_resume_into_the_same_verified_file() {
    let server = Server::new(SIZE, Mode::Exact);
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("resumed.bin");
    let mut request =
        validate_request(&server.url, target.clone(), ReplacementPolicy::Deny).unwrap();
    request.options.max_connections = 4;
    request.options.temporary_directory = Some(dir.path().to_path_buf());
    let engine = spawn_engine_in_memory(Arc::new(|| {}));
    engine.try_send(Command::Submit(request)).unwrap();
    let deadline = Instant::now() + Duration::from_secs(20);
    let mut pause_sent = false;
    let mut paused = false;
    let completed = loop {
        let mut result = None;
        for event in engine.drain_events() {
            if let Event::JobChanged(job) = event {
                if job.id == JobId(1)
                    && job.state == JobState::Running
                    && job.received >= 64 * 1024
                    && !pause_sent
                {
                    engine.try_send(Command::Pause(JobId(1))).unwrap();
                    pause_sent = true;
                }
                if job.id == JobId(1) && job.state == JobState::Paused && !paused {
                    paused = true;
                    assert!(!target.exists());
                    assert!(std::fs::read_dir(dir.path()).unwrap().any(|entry| {
                        entry
                            .unwrap()
                            .path()
                            .extension()
                            .is_some_and(|ext| ext == "seg")
                    }));
                    engine.try_send(Command::Resume(JobId(1))).unwrap();
                }
                if job.id == JobId(1) && job.state.is_terminal() {
                    result = Some(job);
                }
            }
        }
        if let Some(job) = result {
            break job;
        }
        assert!(
            Instant::now() < deadline,
            "paused range transfer did not finish"
        );
        thread::sleep(Duration::from_millis(5));
    };
    assert!(pause_sent && paused);
    assert_eq!(
        completed.state,
        JobState::Completed,
        "{:?}",
        completed.error
    );
    let bytes = std::fs::read(target).unwrap();
    assert_eq!(bytes.len(), SIZE as usize);
    assert!(bytes
        .iter()
        .enumerate()
        .all(|(index, byte)| *byte == byte_at(index as u64)));
    engine.try_send(Command::Shutdown).unwrap();
    engine.join().unwrap();
}

#[test]
fn last_modified_ranges_are_rechecked_before_publication() {
    let server = Server::new(SIZE, Mode::LastModified);
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("modified.bin");
    let done = download(&server, target.clone(), 4);
    assert_eq!(done.state, JobState::Completed, "{:?}", done.error);
    let bytes = std::fs::read(target).unwrap();
    assert_eq!(bytes.len(), SIZE as usize);
    assert!(bytes
        .iter()
        .enumerate()
        .all(|(index, byte)| *byte == byte_at(index as u64)));
}

#[test]
fn speed_limit_applies_to_combined_range_progress() {
    let server = Server::new(SIZE, Mode::Exact);
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("limited.bin");
    let mut request =
        validate_request(&server.url, target.clone(), ReplacementPolicy::Deny).unwrap();
    request.options.max_connections = 4;
    request.options.temporary_directory = Some(dir.path().to_path_buf());
    let engine = spawn_engine_in_memory(Arc::new(|| {}));
    engine.try_send(Command::Submit(request)).unwrap();
    let limit = 256 * 1024;
    engine
        .try_send(Command::SetSpeedLimit(JobId(1), Some(limit)))
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    let first = loop {
        let mut received = None;
        for event in engine.drain_events() {
            if let Event::JobChanged(job) = event {
                if job.id == JobId(1) && job.received > 0 {
                    received = Some(job.received);
                }
                assert_ne!(job.state, JobState::Failed, "{:?}", job.error);
            }
        }
        if let Some(received) = received {
            break received;
        }
        assert!(Instant::now() < deadline, "limited transfer never started");
        thread::sleep(Duration::from_millis(5));
    };
    let started = Instant::now();
    thread::sleep(Duration::from_millis(400));
    let mut latest = first;
    for event in engine.drain_events() {
        if let Event::JobChanged(job) = event {
            if job.id == JobId(1) {
                latest = latest.max(job.received);
            }
        }
    }
    let allowed = (limit as f64 * (started.elapsed().as_secs_f64() + 0.1)) as u64;
    assert!(
        latest - first <= allowed,
        "combined ranges exceeded the speed limit"
    );
    engine
        .try_send(Command::SetSpeedLimit(JobId(1), None))
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(20);
    let done = loop {
        let mut result = None;
        for event in engine.drain_events() {
            if let Event::JobChanged(job) = event {
                if job.id == JobId(1) && job.state.is_terminal() {
                    result = Some(job);
                }
            }
        }
        if let Some(job) = result {
            break job;
        }
        assert!(
            Instant::now() < deadline,
            "limit removal did not finish the transfer"
        );
        thread::sleep(Duration::from_millis(5));
    };
    assert_eq!(done.state, JobState::Completed, "{:?}", done.error);
    assert!(server.counts.peak.load(Ordering::SeqCst) >= 2);
    assert_eq!(std::fs::metadata(target).unwrap().len(), SIZE);
    engine.try_send(Command::Shutdown).unwrap();
    engine.join().unwrap();
}
