use super::*;
use crate::{
    framing::MAX_FRAME,
    protocol::{ReplyStatus, RequestId},
};
use std::{
    os::windows::io::AsRawHandle,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{self, Receiver},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    net::windows::named_pipe::{ClientOptions, NamedPipeClient, NamedPipeServer, ServerOptions},
    runtime::Runtime,
    task::JoinSet,
};
use windows::Win32::Foundation::HANDLE;
fn runtime() -> io::Result<Runtime> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
}
async fn read<R: AsyncRead + Unpin>(stream: &mut R) -> io::Result<Vec<u8>> {
    let size = stream.read_u32_le().await? as usize;
    if size == 0 || size > MAX_FRAME {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Invalid frame length",
        ));
    }
    let mut data = vec![0; size];
    stream.read_exact(&mut data).await?;
    Ok(data)
}
async fn write<W: AsyncWrite + Unpin>(stream: &mut W, data: &[u8]) -> io::Result<()> {
    if data.is_empty() || data.len() > MAX_FRAME {
        return Err(io::Error::other("Invalid frame length"));
    }
    stream.write_u32_le(data.len() as u32).await?;
    stream.write_all(data).await?;
    stream.flush().await
}
fn create(session: &UserSession, first: bool) -> io::Result<NamedPipeServer> {
    let security = SecurityDescriptor::new(session)?;
    let mut attributes = security.attributes();
    unsafe {
        ServerOptions::new()
            .first_pipe_instance(first)
            .reject_remote_clients(true)
            .max_instances(65)
            .create_with_security_attributes_raw(
                session.pipe_name(),
                (&mut attributes as *mut windows::Win32::Security::SECURITY_ATTRIBUTES).cast(),
            )
    }
}
pub struct PipeServer {
    stop: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}
impl PipeServer {
    pub fn start(session: UserSession, router: Arc<dyn RequestRouter>) -> io::Result<Self> {
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        let (ready, rx) = mpsc::sync_channel(1);
        let thread = thread::spawn(move || {
            let result = runtime();
            let rt = match result {
                Ok(rt) => rt,
                Err(e) => {
                    let _ = ready.send(Err(e));
                    return;
                }
            };
            rt.block_on(async move {
            let mut pipe=match create(&session,true) { Ok(p)=>p,Err(e)=>{let _=ready.send(Err(e));return;} }; let _=ready.send(Ok(())); let mut clients=JoinSet::new();
            while !flag.load(Ordering::Acquire) {
                tokio::select! { connection=pipe.connect()=> {
                    if connection.is_err() { break; }
                    let next=match create(&session,false) { Ok(p)=>p,Err(_)=>break }; let connected=std::mem::replace(&mut pipe,next);
                    if clients.len()>=63 { drop(connected); continue; }
                    let session=session.clone(); let router=router.clone(); clients.spawn(async move { let _=tokio::time::timeout(Duration::from_secs(122),serve(connected,session,router)).await; });
                }, _=tokio::time::sleep(Duration::from_millis(50))=>{}, _=clients.join_next(),if !clients.is_empty()=>{} }
            }
            clients.abort_all(); while clients.join_next().await.is_some() {}
        });
        });
        rx.recv()
            .map_err(|_| io::Error::other("Pipe startup stopped"))??;
        Ok(Self {
            stop,
            thread: Some(thread),
        })
    }
}
impl Drop for PipeServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
async fn serve(
    mut pipe: NamedPipeServer,
    session: UserSession,
    router: Arc<dyn RequestRouter>,
) -> io::Result<()> {
    let bytes = tokio::time::timeout(Duration::from_secs(10), read(&mut pipe)).await??;
    session.verify_client(HANDLE(pipe.as_raw_handle()))?;
    let (id, operation) = Operation::decode(&bytes).map_err(io::Error::other)?;
    let (tx, rx) = mpsc::sync_channel(4);
    router.dispatch(operation, tx)?;
    loop {
        match rx.try_recv() {
            Ok(mut reply) => {
                reply.id = id.clone();
                let pending = reply.result == ReplyStatus::Pending;
                write(&mut pipe, &serde_json::to_vec(&reply)?).await?;
                if !pending {
                    return Ok(());
                }
            }
            Err(mpsc::TryRecvError::Empty) => tokio::time::sleep(Duration::from_millis(10)).await,
            Err(_) => return Err(io::Error::other("Reply unavailable")),
        }
    }
}
pub struct PipeClient {
    connection: Option<(Runtime, NamedPipeClient)>,
}
impl PipeClient {
    pub fn connect(session: &UserSession, timeout: Duration) -> io::Result<Self> {
        let rt = runtime()?;
        let pipe = rt.block_on(async {
            let deadline = Instant::now() + timeout;
            loop {
                match ClientOptions::new().open(session.pipe_name()) {
                    Ok(pipe) => {
                        session.verify_server(HANDLE(pipe.as_raw_handle()))?;
                        return Ok(pipe);
                    }
                    Err(e)
                        if Instant::now() < deadline
                            && matches!(e.raw_os_error(), Some(2 | 231)) =>
                    {
                        tokio::time::sleep(Duration::from_millis(25)).await
                    }
                    Err(e) => return Err(e),
                }
            }
        })?;
        Ok(Self {
            connection: Some((rt, pipe)),
        })
    }
    pub fn request(&mut self, operation: Operation) -> Result<ReplyStream, BridgeError> {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        let id = match &operation {
            Operation::Capture(c) => c.id.clone(),
            _ => RequestId::parse(&format!(
                "00000000-0000-4000-8000-{:012x}",
                NEXT.fetch_add(1, Ordering::Relaxed) & 0xffffffffffff
            ))
            .map_err(io::Error::other)?,
        };
        let data = operation.encode(id.clone()).map_err(io::Error::other)?;
        let (rt, mut pipe) = self
            .connection
            .take()
            .ok_or_else(|| io::Error::other("Connection already used"))?;
        if matches!(operation, Operation::Capture(_) | Operation::Activate) {
            // Chrome's native child can delegate foreground permission to the
            // verified GUI process before its UI thread receives the request.
            // Do not grant ASFW_ANY or change system foreground-lock settings.
            unsafe {
                let mut server_pid = 0;
                if windows::Win32::System::Pipes::GetNamedPipeServerProcessId(
                    HANDLE(pipe.as_raw_handle()),
                    &mut server_pid,
                )
                .is_ok()
                {
                    let _ = windows::Win32::UI::WindowsAndMessaging::AllowSetForegroundWindow(
                        server_pid,
                    );
                }
            }
        }
        let (tx, rx) = mpsc::sync_channel(4);
        thread::spawn(move || {
            rt.block_on(async move {
                let result = tokio::time::timeout(Duration::from_secs(122), async {
                    write(&mut pipe, &data).await?;
                    loop {
                        let reply: Reply = serde_json::from_slice(&read(&mut pipe).await?)?;
                        if reply.v != 1 || reply.id != id {
                            return Err(io::Error::other("Mismatched reply"));
                        }
                        let pending = reply.result == ReplyStatus::Pending;
                        if tx.try_send(Ok(reply)).is_err() {
                            return Ok(());
                        }
                        if !pending {
                            return Ok(());
                        }
                    }
                })
                .await;
                match result {
                    Ok(Ok(())) => {}
                    Ok(Err(e)) => {
                        let _ = tx.try_send(Err(e));
                    }
                    Err(_) => {
                        let _ = tx.try_send(Err(io::Error::new(
                            io::ErrorKind::TimedOut,
                            "Pipe reply expired",
                        )));
                    }
                }
            });
        });
        Ok(ReplyStream(rx))
    }
}
pub struct ReplyStream(Receiver<io::Result<Reply>>);
impl ReplyStream {
    pub fn recv(&self) -> io::Result<Reply> {
        self.0
            .recv_timeout(Duration::from_secs(123))
            .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "Pipe disconnected"))?
    }
}
