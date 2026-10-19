mod support;
use rusty_engine::{
    transfer::{controlled_transfer, TransferControl, TransferOutcome},
    validate_request, ReplacementPolicy,
};
use std::{
    io::Write,
    net::TcpStream,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
use tokio_util::sync::CancellationToken;

// The actual HTTP request, partial file and final bytes are exercised. A wrong
// Range offset, omitted If-Range, or appending a fallback 200 corrupts this test.
fn respond(mut stream: TcpStream) {
    let req = support::request(&mut stream);
    let path = req.split_whitespace().nth(1).unwrap_or("/");
    let range = req.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        if !name.eq_ignore_ascii_case("range") {
            return None;
        }
        value
            .trim()
            .strip_prefix("bytes=")?
            .strip_suffix('-')?
            .parse::<usize>()
            .ok()
    });
    let mut body = support::bytes();
    if path == "/changed" && range.is_some() {
        body.fill(19);
    }
    let start = if path == "/ignored" || path == "/changed" {
        0
    } else {
        range.unwrap_or(0)
    };
    let partial = range.is_some() && !matches!(path, "/ignored" | "/changed");
    if partial {
        assert!(req
            .lines()
            .any(|line| line
                .split_once(':')
                .is_some_and(|(name, value)| name.eq_ignore_ascii_case("if-range")
                    && value.trim() == "\"v1\"")));
        let offset = if path == "/bad-range" {
            start + 1
        } else {
            start
        };
        let _ = write!(
            stream,
            "HTTP/1.1 206 Partial Content\r\nContent-Range: bytes {offset}-{}/{}\r\n",
            body.len() - 1,
            body.len()
        );
    } else {
        let _ = stream.write_all(b"HTTP/1.1 200 OK\r\n");
    }
    let tag = if path == "/weak" {
        "W/\"v1\""
    } else if path == "/changed" && range.is_some() {
        "\"v2\""
    } else {
        "\"v1\""
    };
    let _ = write!(
        stream,
        "ETag: {tag}\r\nAccept-Ranges: bytes\r\nConnection: close\r\n"
    );
    if path != "/unknown" {
        let _ = write!(stream, "Content-Length: {}\r\n", body.len() - start);
    }
    let _ = stream.write_all(b"\r\n");
    for part in body[start..].chunks(1024) {
        if stream.write_all(part).is_err() {
            break;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}

#[tokio::test]
async fn pause_reconnects_with_verified_range_or_safely_restarts_without_it() {
    let fixture = support::Fixture::new(respond);
    for path in [
        "/range",
        "/ignored",
        "/changed",
        "/weak",
        "/unknown",
        "/bad-range",
    ] {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("file.bin");
        let request = validate_request(
            &format!("{}{path}", fixture.url),
            target.clone(),
            ReplacementPolicy::Deny,
        )
        .unwrap();
        let control = TransferControl::new(CancellationToken::new());
        let actor = control.clone();
        let issued = Arc::new(AtomicBool::new(false));
        let paused = Arc::new(tokio::sync::Notify::new());
        let wake = paused.clone();
        let received = Arc::new(Mutex::new(0));
        let latest = received.clone();
        let task = tokio::spawn(controlled_transfer(
            request,
            control.clone(),
            Arc::new(move |p| {
                *latest.lock().unwrap() = p.received;
                if p.received >= 1024 && !issued.swap(true, Ordering::SeqCst) {
                    assert!(actor.try_pause());
                }
                if p.paused {
                    wake.notify_one();
                }
            }),
        ));
        tokio::time::timeout(Duration::from_secs(3), paused.notified())
            .await
            .unwrap();
        let held = *received.lock().unwrap();
        assert!(held >= 1024);
        tokio::time::sleep(Duration::from_millis(40)).await;
        assert_eq!(*received.lock().unwrap(), held);
        assert!(!target.exists());
        assert!(control.try_resume());
        let result = tokio::time::timeout(Duration::from_secs(3), task)
            .await
            .unwrap()
            .unwrap();
        if path == "/bad-range" {
            assert!(result.is_err());
            assert!(!target.exists());
        } else {
            assert_eq!(result.unwrap(), TransferOutcome::Completed { bytes: 65536 });
            let expected = if path == "/changed" {
                vec![19; 65536]
            } else {
                support::bytes()
            };
            assert_eq!(std::fs::read(&target).unwrap(), expected, "{path}");
        }
        assert_eq!(
            std::fs::read_dir(dir.path()).unwrap().count(),
            usize::from(target.exists())
        );
    }
}

#[tokio::test]
async fn cancel_while_paused_cleans_owned_partial() {
    let fixture = support::Fixture::new(respond);
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("file.bin");
    let request = validate_request(&fixture.url, target.clone(), ReplacementPolicy::Deny).unwrap();
    let control = TransferControl::new(CancellationToken::new());
    control.try_pause();
    let paused = Arc::new(tokio::sync::Notify::new());
    let wake = paused.clone();
    let task = tokio::spawn(controlled_transfer(
        request,
        control.clone(),
        Arc::new(move |p| {
            if p.paused {
                wake.notify_one();
            }
        }),
    ));
    tokio::time::timeout(Duration::from_secs(3), paused.notified())
        .await
        .unwrap();
    control.try_cancel();
    assert_eq!(task.await.unwrap().unwrap(), TransferOutcome::Cancelled);
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
}

#[tokio::test]
async fn live_limit_can_be_removed_without_restarting_the_transfer() {
    let fixture = support::Fixture::new(respond);
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("file.bin");
    let request = validate_request(&fixture.url, target.clone(), ReplacementPolicy::Deny).unwrap();
    let control = TransferControl::new(CancellationToken::new());
    control.set_speed_limit(Some(1024));
    let received = Arc::new(Mutex::new(0));
    let latest = received.clone();
    let start = tokio::time::Instant::now();
    let task = tokio::spawn(controlled_transfer(
        request,
        control.clone(),
        Arc::new(move |p| {
            *latest.lock().unwrap() = p.received;
        }),
    ));
    tokio::time::sleep(Duration::from_millis(350)).await;
    assert!(!task.is_finished());
    let elapsed = start.elapsed().as_secs_f64();
    assert!(*received.lock().unwrap() as f64 <= 1024.0 * elapsed);
    control.set_speed_limit(None);
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(3), task)
            .await
            .unwrap()
            .unwrap()
            .unwrap(),
        TransferOutcome::Completed { bytes: 65536 }
    );
    assert_eq!(std::fs::read(target).unwrap(), support::bytes());
}

#[test]
fn paused_job_releases_slot_and_resume_waits_its_turn_then_shutdown_cleans_all() {
    use rusty_engine::{spawn_engine, Command, EngineHandle, Event, JobId, JobState};
    fn wait(engine: &EngineHandle, predicate: impl Fn(&[Event]) -> bool) {
        let deadline = std::time::Instant::now() + Duration::from_secs(4);
        let mut events = Vec::new();
        while !predicate(&events) {
            events.extend(engine.drain_events());
            assert!(
                std::time::Instant::now() < deadline,
                "Missing event: {events:?}"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    let fixture = support::Fixture::new(respond);
    let dir = tempfile::tempdir().unwrap();
    let engine = spawn_engine(Arc::new(|| {}));
    for id in 1..=3 {
        let request = validate_request(
            &fixture.url,
            dir.path().join(format!("{id}.bin")),
            ReplacementPolicy::Deny,
        )
        .unwrap();
        engine.try_send(Command::Submit(request)).unwrap();
        engine
            .try_send(Command::SetSpeedLimit(JobId(id), Some(1024)))
            .unwrap();
    }
    wait(&engine, |events| {
        events.iter().any(|e| matches!(e, Event::JobChanged(s) if s.id == JobId(2) && s.state == JobState::Running))
    });
    engine.try_send(Command::Pause(JobId(1))).unwrap();
    wait(&engine, |events| {
        events.iter().any(|e| matches!(e, Event::JobChanged(s) if s.id == JobId(1) && s.state == JobState::Paused)) && events.iter().any(|e| matches!(e, Event::JobChanged(s) if s.id == JobId(3) && s.state == JobState::Running))
    });
    engine.try_send(Command::Resume(JobId(1))).unwrap();
    wait(&engine, |events| {
        events.iter().any(|e| matches!(e, Event::JobChanged(s) if s.id == JobId(1) && s.state == JobState::Waiting))
    });
    engine.try_send(Command::Cancel(JobId(3))).unwrap();
    wait(&engine, |events| {
        events.iter().any(|e| matches!(e, Event::JobChanged(s) if s.id == JobId(1) && s.state == JobState::Running && s.received > 0))
    });
    engine.try_send(Command::Pause(JobId(1))).unwrap();
    wait(&engine, |events| {
        events.iter().any(|e| matches!(e, Event::JobChanged(s) if s.id == JobId(1) && s.state == JobState::Paused))
    });
    engine.try_send(Command::Shutdown).unwrap();
    wait(&engine, |events| {
        events.iter().any(|e| matches!(e, Event::ShutdownComplete))
    });
    engine.join().unwrap();
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
}
