use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

pub struct Fixture {
    pub url: String,
    stop: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}
impl Fixture {
    pub fn new(handler: fn(TcpStream)) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        listener.set_nonblocking(true).unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        let thread = thread::spawn(move || {
            while !flag.load(Ordering::Acquire) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        stream.set_nonblocking(false).unwrap();
                        handler(stream);
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5))
                    }
                    Err(_) => break,
                }
            }
        });
        Self {
            url,
            stop,
            thread: Some(thread),
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(t) = self.thread.take() {
            t.join().unwrap();
        }
    }
}

pub fn request(stream: &mut TcpStream) -> String {
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    let mut data = Vec::new();
    let mut byte = [0];
    while data.len() < 8192 && stream.read(&mut byte).unwrap_or(0) != 0 {
        data.push(byte[0]);
        if data.ends_with(b"\r\n\r\n") {
            break;
        }
    }
    String::from_utf8_lossy(&data).into_owned()
}
pub fn bytes() -> Vec<u8> {
    (0..65536)
        .map(|i| ((i * 31 + i / 251) % 256) as u8)
        .collect()
}
pub fn respond(mut stream: TcpStream) {
    let req = request(&mut stream);
    let path = req.split_whitespace().nth(1).unwrap_or("/");
    match path {
        "/auth" => {
            let accepted = req.lines().any(|line| {
                line.strip_prefix("authorization: ")
                    .or_else(|| line.strip_prefix("Authorization: "))
                    == Some("Basic dXNlcjpwYXNz")
            });
            if accepted {
                let _ = stream.write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok",
                );
            } else {
                let _ = stream.write_all(
                    b"HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                );
            }
        }
        "/redirect" => {
            let _ = stream.write_all(b"HTTP/1.1 302 Found\r\nLocation: /data\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
        }
        "/loop" => {
            let _ = stream.write_all(b"HTTP/1.1 302 Found\r\nLocation: /loop\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
        }
        "/404" | "/503" => {
            let status = if path == "/404" {
                "404 Not Found"
            } else {
                "503 Unavailable"
            };
            let _ = write!(
                stream,
                "HTTP/1.1 {status}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            );
        }
        "/empty" => {
            let _ = stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
        }
        "/encoded" => {
            let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Encoding: gzip\r\nContent-Length: 3\r\nConnection: close\r\n\r\nabc");
        }
        "/broken" => {
            let _ = stream.write_all(
                b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\nConnection: close\r\n\r\nshort",
            );
        }
        "/partial" => {
            let _ = stream.write_all(b"HTTP/1.1 206 Partial Content\r\nContent-Length: 3\r\nContent-Range: bytes 0-2/100\r\nConnection: close\r\n\r\nabc");
        }
        "/slow" => {
            let _ = stream.write_all(
                b"HTTP/1.1 200 OK\r\nContent-Length: 100000\r\nConnection: close\r\n\r\n",
            );
            for _ in 0..50 {
                if stream.write_all(&[7; 100]).is_err() {
                    break;
                }
                thread::sleep(Duration::from_millis(10));
            }
        }
        _ => {
            let data = bytes();
            if path == "/unknown" {
                let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nConnection: close\r\n\r\n");
            } else {
                let _ = write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    data.len()
                );
            }
            let _ = stream.write_all(&data);
        }
    }
}
