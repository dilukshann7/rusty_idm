use crate::config::{HostConfig, HostError};
use std::{
    io::{Read, Write},
    process::Command,
};
pub fn gui_command(config: &HostConfig) -> Command {
    let mut command = Command::new(&config.gui_exe);
    command.arg("--browser-integration");
    command
}
#[cfg(windows)]
pub fn run_host<R: Read + Send, W: Write + Send>(
    input: &mut R,
    output: &mut W,
    config: &HostConfig,
    caller_origin: &str,
) -> Result<(), HostError> {
    use rusty_browser_bridge::{framing::*, protocol::*, windows::*};
    use std::{
        sync::{
            atomic::{AtomicBool, AtomicUsize, Ordering},
            mpsc, Arc, Mutex,
        },
        time::{Duration, Instant},
    };
    config.validate_origin(caller_origin)?;
    let session = UserSession::current()?;
    // One bounded output writer keeps stdout exclusively framed; input keeps reading while captures wait.
    std::thread::scope(|scope| {
        let (tx, rx) = mpsc::sync_channel::<Reply>(128);
        let disconnected = Arc::new(AtomicBool::new(false));
        let flag = disconnected.clone();
        let writer = scope.spawn(move || -> Result<(), HostError> {
            for reply in rx {
                if let Err(e) = write_frame(output, &serde_json::to_vec(&reply)?) {
                    flag.store(true, Ordering::Release);
                    return Err(e);
                }
            }
            Ok(())
        });
        let active = Arc::new(AtomicUsize::new(0));
        let launch = Arc::new(Mutex::new(false));
        let mut workers = Vec::new();
        let mut input_error = None;
        loop {
            workers.retain(|thread: &std::thread::ScopedJoinHandle<'_, ()>| !thread.is_finished());
            let bytes = match read_frame(input) {
                Ok(Some(b)) => b,
                Ok(None) => break,
                Err(e) => {
                    input_error = Some(e);
                    break;
                }
            };
            let (id, operation) = match Operation::decode(&bytes) {
                Ok(value) => value,
                Err(_) => {
                    input_error = Some(std::io::Error::other("Invalid native request"));
                    break;
                }
            };
            if active.load(Ordering::Acquire) >= 64 {
                let _ = tx.send(Reply::new(
                    id,
                    ReplyStatus::Declined {
                        code: DeclineCode::Busy,
                    },
                ));
                continue;
            }
            active.fetch_add(1, Ordering::AcqRel);
            let active = active.clone();
            let tx = tx.clone();
            let session = session.clone();
            let config = config.clone();
            let launch = launch.clone();
            let disconnected = disconnected.clone();
            workers.push(scope.spawn(move || {
                let result = (|| -> Result<(), HostError> {
                    let ping = matches!(operation, Operation::Ping);
                    let start = Instant::now();
                    let mut client = match PipeClient::connect(&session, Duration::from_millis(100))
                    {
                        Ok(client) => client,
                        Err(_) if ping => {
                            tx.send(Reply::new(
                                id.clone(),
                                ReplyStatus::Ok {
                                    host_version: env!("CARGO_PKG_VERSION").into(),
                                    gui_running: false,
                                },
                            ))
                            .map_err(|_| std::io::Error::other("Output closed"))?;
                            return Ok(());
                        }
                        Err(_) => {
                            {
                                let mut launched = launch.lock().unwrap_or_else(|p| p.into_inner());
                                if !*launched {
                                    use std::os::windows::process::CommandExt;
                                    gui_command(&config).creation_flags(0x08000000).spawn()?;
                                    *launched = true;
                                }
                            }
                            PipeClient::connect(
                                &session,
                                Duration::from_secs(10).saturating_sub(start.elapsed()),
                            )?
                        }
                    };
                    let replies = client.request(operation)?;
                    loop {
                        if disconnected.load(Ordering::Acquire) {
                            return Ok(());
                        }
                        let mut reply = replies.recv()?;
                        reply.id = id.clone();
                        let pending = reply.result == ReplyStatus::Pending;
                        tx.send(reply)
                            .map_err(|_| std::io::Error::other("Output closed"))?;
                        if !pending {
                            break;
                        }
                    }
                    Ok(())
                })();
                if result.is_err() {
                    let _ = tx.send(Reply::new(
                        id,
                        ReplyStatus::Declined {
                            code: DeclineCode::EngineUnavailable,
                        },
                    ));
                }
                active.fetch_sub(1, Ordering::AcqRel);
            }));
        }
        for worker in workers {
            let _ = worker.join();
        }
        drop(tx);
        writer
            .join()
            .map_err(|_| std::io::Error::other("Output stopped"))??;
        if let Some(e) = input_error {
            return Err(e);
        }
        Ok(())
    })
}
