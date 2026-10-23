#[allow(dead_code)]
mod support;

use rusty_browser_bridge::protocol::{RequestHeader, RequestHop};
use rusty_engine::{
    spawn_engine_in_memory, validate_request, Command, Event, JobSnapshot, JobState,
    ReplacementPolicy,
};
use std::{
    io::Write,
    net::{TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};

struct Server {
    url: String,
    requests: Arc<Mutex<Vec<String>>>,
    stop: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
}
impl Server {
    fn new(handler: impl Fn(&str, usize, &mut TcpStream) + Send + 'static) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let url = format!("http://{}/file", listener.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(Vec::<String>::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let seen = requests.clone();
        let flag = stop.clone();
        let worker = thread::spawn(move || {
            while !flag.load(Ordering::Acquire) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        stream.set_nonblocking(false).unwrap();
                        stream
                            .set_write_timeout(Some(Duration::from_secs(2)))
                            .unwrap();
                        let request = support::request(&mut stream);
                        let index = {
                            let mut seen = seen.lock().unwrap();
                            let index = seen.len();
                            seen.push(request.clone());
                            index
                        };
                        handler(&request, index, &mut stream);
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(2))
                    }
                    Err(_) => break,
                }
            }
        });
        Self {
            url,
            requests,
            stop,
            worker: Some(worker),
        }
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        self.worker.take().unwrap().join().unwrap();
    }
}

fn download(
    server: &Server,
    target: &std::path::Path,
    context: bool,
    retries: u8,
) -> Vec<JobSnapshot> {
    let mut request =
        validate_request(&server.url, target.to_path_buf(), ReplacementPolicy::Deny).unwrap();
    request.options.max_connections = 1;
    request.options.retry_policy.max_retries = retries;
    request.options.retry_policy.delay_seconds = 1;
    if context {
        request = request
            .with_browser_context(vec![RequestHop {
                url: server.url.clone(),
                headers: vec![RequestHeader {
                    name: "Cookie".into(),
                    value: "session=test".into(),
                }],
            }])
            .unwrap();
    }
    run(request)
}

fn run(request: rusty_engine::DownloadRequest) -> Vec<JobSnapshot> {
    let engine = spawn_engine_in_memory(Arc::new(|| {}));
    engine.try_send(Command::Submit(request)).unwrap();
    let deadline = Instant::now() + Duration::from_secs(15);
    let mut snapshots = vec![];
    loop {
        snapshots.extend(
            engine
                .drain_events()
                .into_iter()
                .filter_map(|event| match event {
                    Event::JobChanged(job) => Some(job),
                    _ => None,
                }),
        );
        if snapshots.last().is_some_and(|job| job.state.is_terminal()) {
            break;
        }
        assert!(Instant::now() < deadline, "recovery did not finish");
        thread::sleep(Duration::from_millis(5));
    }
    engine.client().request_shutdown();
    engine.join().unwrap();
    snapshots
}

#[test]
fn interrupted_streams_resume_saved_bytes_and_keep_browser_credentials_in_memory() {
    for context in [false, true] {
        let server = Server::new(|request, index, stream| {
            if index == 0 {
                stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 12\r\nETag: \"v1\"\r\nConnection: close\r\n\r\nhello").unwrap();
            } else {
                assert!(request.to_ascii_lowercase().contains("range: bytes=5-"));
                assert!(request.to_ascii_lowercase().contains("if-range: \"v1\""));
                stream.write_all(b"HTTP/1.1 206 Partial Content\r\nContent-Length: 7\r\nContent-Range: bytes 5-11/12\r\nETag: \"v1\"\r\nConnection: close\r\n\r\n world!").unwrap();
            }
        });
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("resume.bin");
        let snapshots = download(&server, &target, context, 3);
        assert_eq!(snapshots.last().unwrap().state, JobState::Completed);
        assert_eq!(std::fs::read(&target).unwrap(), b"hello world!");
        assert!(snapshots
            .iter()
            .any(|job| job.retry.is_some() && job.received == 5));
        assert!(!snapshots.iter().any(|job| job.state == JobState::Failed));
        assert!(snapshots
            .iter()
            .all(|job| job.entry_id == snapshots[0].entry_id
                && job.created_at == snapshots[0].created_at));
        let requests = server.requests.lock().unwrap();
        assert_eq!(requests.len(), 2);
        if context {
            assert!(requests.iter().all(|request| request
                .to_ascii_lowercase()
                .contains("cookie: session=test")));
        }
    }
}

#[test]
fn temporary_http_failure_recovers_for_browser_and_manual_downloads() {
    for context in [false, true] {
        let server = Server::new(|_, index, stream| {
            let response: &[u8] = if index == 0 {
                b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            } else {
                b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok"
            };
            stream.write_all(response).unwrap();
        });
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("http-retry.bin");
        let snapshots = download(&server, &target, context, 3);
        assert_eq!(snapshots.last().unwrap().state, JobState::Completed);
        assert_eq!(server.requests.lock().unwrap().len(), 2);
        assert_eq!(std::fs::read(target).unwrap(), b"ok");
    }
}

#[test]
fn retries_exhaust_exactly_and_zero_disables_recovery() {
    for retries in [0, 3] {
        let server = Server::new(|_, _, stream| {
            stream.write_all(b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").unwrap();
        });
        let dir = tempfile::tempdir().unwrap();
        let snapshots = download(&server, &dir.path().join("failed.bin"), false, retries);
        let failed = snapshots.last().unwrap();
        assert_eq!(failed.state, JobState::Failed);
        assert!(failed.retry.is_none());
        assert!(failed.finished_at.is_some());
        assert_eq!(
            server.requests.lock().unwrap().len(),
            usize::from(retries) + 1
        );
        assert_eq!(
            snapshots
                .iter()
                .filter(|job| job.state == JobState::Failed)
                .count(),
            1
        );
    }
}

#[test]
fn expired_credentials_stop_without_automatic_replay() {
    let server = Server::new(|_, _, stream| {
        stream
            .write_all(b"HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
            .unwrap();
    });
    let dir = tempfile::tempdir().unwrap();
    let snapshots = download(&server, &dir.path().join("auth.bin"), true, 3);
    let failed = snapshots.last().unwrap();
    assert_eq!(failed.state, JobState::Failed);
    assert!(!failed.error.as_ref().unwrap().retryable);
    assert_eq!(server.requests.lock().unwrap().len(), 1);
}

#[test]
fn a_connection_closed_before_response_headers_is_retried() {
    let server = Server::new(|_, index, stream| {
        if index > 0 {
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok")
                .unwrap();
        }
    });
    let dir = tempfile::tempdir().unwrap();
    let snapshots = download(&server, &dir.path().join("header-drop.bin"), false, 3);
    assert_eq!(snapshots.last().unwrap().state, JobState::Completed);
    assert_eq!(server.requests.lock().unwrap().len(), 2);
}

#[test]
fn temporary_http_failure_at_an_intermediate_captured_redirect_hop_is_retried() {
    let server = Server::new(|request, index, stream| {
        let response: &[u8] = if index == 0 {
            b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        } else if request.starts_with("GET /file/redirect ") {
            b"HTTP/1.1 302 Found\r\nLocation: /file\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        } else {
            b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok"
        };
        stream.write_all(response).unwrap();
    });
    let dir = tempfile::tempdir().unwrap();
    let mut request = validate_request(
        &server.url,
        dir.path().join("redirect-retry.bin"),
        ReplacementPolicy::Deny,
    )
    .unwrap()
    .with_browser_context(vec![
        RequestHop {
            url: format!("{}/redirect", server.url),
            headers: vec![],
        },
        RequestHop {
            url: server.url.clone(),
            headers: vec![],
        },
    ])
    .unwrap();
    request.options.max_connections = 1;
    request.options.retry_policy.delay_seconds = 1;
    let snapshots = run(request);
    assert_eq!(snapshots.last().unwrap().state, JobState::Completed);
    assert_eq!(server.requests.lock().unwrap().len(), 3);
}
