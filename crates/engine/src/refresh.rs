//! Verification of a replacement HTTP address before saved bytes are reused.
use crate::{output, storage::Checkpoint, ErrorKind, JobError};
use reqwest::{
    header::{
        HeaderMap, HeaderName, HeaderValue, ACCEPT_ENCODING, CONTENT_ENCODING, CONTENT_RANGE, ETAG,
        IF_RANGE, LAST_MODIFIED, LOCATION, RANGE,
    },
    Client, Response, StatusCode,
};
use rusty_browser_bridge::protocol::RequestHop;
use std::{
    io::{Read, Seek, SeekFrom},
    time::Duration,
};
use url::Url;

#[derive(Clone)]
pub struct RefreshCandidate {
    pub url: String,
    pub source_page: Option<String>,
    pub request_context: Vec<RequestHop>,
}
impl std::fmt::Debug for RefreshCandidate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RefreshCandidate")
            .field("context_hops", &self.request_context.len())
            .finish_non_exhaustive()
    }
}
impl RefreshCandidate {
    pub fn validate(&self) -> Result<Url, JobError> {
        let url =
            Url::parse(&self.url).map_err(|_| invalid("Enter a valid HTTP/HTTPS address."))?;
        if !safe_url(&url) || self.url.len() > 8192 || self.url.chars().any(char::is_control) {
            return Err(invalid("Enter a valid HTTP/HTTPS address."));
        }
        if let Some(page) = &self.source_page {
            let page_url = Url::parse(page).map_err(|_| invalid("Invalid source page."))?;
            if !safe_url(&page_url) || page.len() > 8192 || page.chars().any(char::is_control) {
                return Err(invalid("Invalid source page."));
            }
        }
        if self.request_context.len() > 16 {
            return Err(invalid("Too many browser redirect hops."));
        }
        if !self.request_context.is_empty()
            && (self
                .request_context
                .last()
                .is_none_or(|hop| hop.url != self.url)
                || self.request_context.iter().enumerate().any(|(index, hop)| {
                    self.request_context[..index]
                        .iter()
                        .any(|earlier| earlier.url == hop.url)
                }))
        {
            return Err(invalid("Ambiguous browser redirect context."));
        }
        let mut size = 0usize;
        for hop in &self.request_context {
            let hop_url =
                Url::parse(&hop.url).map_err(|_| invalid("Invalid browser redirect address."))?;
            if !safe_url(&hop_url)
                || hop.url.len() > 8192
                || hop.url.chars().any(char::is_control)
                || hop.headers.len() > 16
            {
                return Err(invalid("Invalid browser request context."));
            }
            size += hop.url.len();
            for header in &hop.headers {
                let name = header.name.to_ascii_lowercase();
                if !matches!(
                    name.as_str(),
                    "cookie" | "authorization" | "referer" | "user-agent"
                ) || header.value.len() > 8192
                    || header.value.chars().any(char::is_control)
                {
                    return Err(invalid("Invalid browser request header."));
                }
                size += name.len() + header.value.len();
            }
        }
        if size > 32768 {
            return Err(invalid("Browser request context is too large."));
        }
        Ok(url)
    }
}

#[derive(Clone)]
pub(crate) struct VerifiedRefresh {
    pub final_url: String,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub total: u64,
}

fn invalid(message: &str) -> JobError {
    JobError::new(ErrorKind::InvalidRequest, message)
}
fn mismatch(message: &str) -> JobError {
    JobError::new(ErrorKind::RestartRequired, message)
}
fn safe_url(url: &Url) -> bool {
    matches!(url.scheme(), "http" | "https")
        && url.host_str().is_some()
        && url.username().is_empty()
        && url.password().is_none()
}
pub(crate) fn client() -> Result<Client, JobError> {
    Client::builder()
        .user_agent("RustyDownloadManager/0.2.1")
        .redirect(reqwest::redirect::Policy::none())
        .no_gzip()
        .no_brotli()
        .no_deflate()
        .no_zstd()
        .retry(reqwest::retry::never())
        .connect_timeout(Duration::from_secs(30))
        .build()
        .map_err(|_| JobError::new(ErrorKind::Network, "Cannot initialize the HTTP client."))
}
pub(crate) fn sample_ranges(received: u64) -> Vec<(u64, u64)> {
    if received == 0 {
        return Vec::new();
    }
    let width = received.min(65_536);
    let middle = received.saturating_sub(width) / 2;
    let mut ranges = vec![
        (0, width - 1),
        (middle, middle + width - 1),
        (received - width, received - 1),
    ];
    ranges.sort_unstable();
    ranges.dedup();
    ranges
}
pub(crate) fn exact_range(
    headers: &HeaderMap,
    start: u64,
    end: u64,
    total: u64,
    length: Option<u64>,
) -> bool {
    let expected = format!("bytes {start}-{end}/{total}");
    headers.get(CONTENT_RANGE).and_then(|v| v.to_str().ok()) == Some(expected.as_str())
        && length.is_some_and(|n| n == end - start + 1)
}
pub(crate) fn strong_etag(headers: &HeaderMap) -> Option<String> {
    let value = headers.get(ETAG)?.to_str().ok()?;
    let opaque = value.strip_prefix('"')?.strip_suffix('"')?;
    if opaque
        .bytes()
        .all(|b| b == 0x21 || (0x23..=0x7e).contains(&b) || b >= 0x80)
    {
        Some(value.to_owned())
    } else {
        None
    }
}
fn identity_encoding(headers: &HeaderMap) -> bool {
    headers
        .get_all(CONTENT_ENCODING)
        .iter()
        .all(|v| v.to_str().is_ok_and(|s| s.eq_ignore_ascii_case("identity")))
}

/// Follow only observed redirect URLs when browser headers are present.
pub(crate) async fn fetch_range(
    client: &Client,
    url: &Url,
    context: &[RequestHop],
    start: u64,
    end: Option<u64>,
    if_range: Option<&str>,
) -> Result<Response, JobError> {
    fetch_scoped(client, url, context, Some((start, end, if_range))).await
}

pub(crate) async fn fetch_get(
    client: &Client,
    url: &Url,
    context: &[RequestHop],
) -> Result<Response, JobError> {
    fetch_scoped(client, url, context, None).await
}

async fn fetch_scoped(
    client: &Client,
    url: &Url,
    context: &[RequestHop],
    range: Option<(u64, Option<u64>, Option<&str>)>,
) -> Result<Response, JobError> {
    let mut current = context
        .first()
        .map(|hop| Url::parse(&hop.url).map_err(|_| invalid("Invalid browser request address.")))
        .transpose()?
        .unwrap_or_else(|| url.clone());
    for hop_index in 0..=16 {
        let mut builder = client
            .get(current.clone())
            .header(ACCEPT_ENCODING, "identity");
        if let Some((start, end, if_range)) = range {
            builder = builder.header(
                RANGE,
                match end {
                    Some(end) => format!("bytes={start}-{end}"),
                    None => format!("bytes={start}-"),
                },
            );
            if let Some(value) = if_range {
                builder = builder.header(IF_RANGE, value);
            }
        }
        if !context.is_empty() {
            let hop = context
                .get(hop_index)
                .filter(|h| h.url == current.as_str())
                .ok_or_else(|| {
                    mismatch("The server redirected to an address not observed in the browser.")
                })?;
            for header in &hop.headers {
                let name = HeaderName::from_bytes(header.name.as_bytes())
                    .map_err(|_| invalid("Invalid browser request header."))?;
                let value = HeaderValue::from_str(&header.value)
                    .map_err(|_| invalid("Invalid browser request header."))?;
                builder = builder.header(name, value);
            }
        }
        let response = tokio::time::timeout(Duration::from_secs(30), builder.send())
            .await
            .map_err(|_| JobError::new(ErrorKind::Network, "Refresh request timed out."))?
            .map_err(|_| {
                JobError::new(
                    ErrorKind::Network,
                    "Cannot reach the refreshed download address.",
                )
            })?;
        if response.status().is_redirection() {
            let next = response
                .headers()
                .get(LOCATION)
                .and_then(|v| v.to_str().ok())
                .and_then(|location| current.join(location).ok())
                .ok_or_else(|| mismatch("The server returned an invalid redirect."))?;
            if !safe_url(&next) {
                return Err(mismatch("The server redirected to an unsafe address."));
            }
            if !context.is_empty()
                && context
                    .get(hop_index + 1)
                    .is_none_or(|hop| hop.url != next.as_str())
            {
                return Err(mismatch(
                    "The server redirect chain differs from the browser capture.",
                ));
            }
            current = next;
            continue;
        }
        if !context.is_empty() && (current != *url || hop_index + 1 != context.len()) {
            return Err(mismatch("The browser download no longer reaches its observed final address. Capture it again."));
        }
        return Ok(response);
    }
    Err(mismatch("The refreshed address redirected too many times."))
}

pub(crate) async fn verify_remote_samples(
    client: &Client,
    candidate: &RefreshCandidate,
    total: u64,
    samples: &[(u64, Vec<u8>)],
) -> Result<VerifiedRefresh, JobError> {
    let url = candidate.validate()?;
    if samples.is_empty() {
        return Err(mismatch("There are no saved bytes to resume."));
    }
    let mut final_url: Option<String> = None;
    let mut validator: Option<Option<String>> = None;
    let mut modified: Option<Option<String>> = None;
    for (start, expected) in samples {
        let end = start + expected.len() as u64 - 1;
        let mut response = fetch_range(
            client,
            &url,
            &candidate.request_context,
            *start,
            Some(end),
            None,
        )
        .await?;
        if response.status() == StatusCode::UNAUTHORIZED
            || response.status() == StatusCode::FORBIDDEN
        {
            return Err(JobError::new(
                ErrorKind::Http,
                "Browser authorization expired; capture the download again.",
            ));
        }
        if response.status() != StatusCode::PARTIAL_CONTENT
            || !identity_encoding(response.headers())
            || !exact_range(
                response.headers(),
                *start,
                end,
                total,
                response.content_length(),
            )
        {
            return Err(mismatch(
                "The server did not return the required exact byte range and total size.",
            ));
        }
        let response_url = response.url().as_str().to_owned();
        let response_etag = strong_etag(response.headers());
        let response_modified = response
            .headers()
            .get(LAST_MODIFIED)
            .and_then(|h| h.to_str().ok())
            .map(str::to_owned);
        if final_url
            .as_ref()
            .is_some_and(|value| value != &response_url)
            || validator
                .as_ref()
                .is_some_and(|value| value != &response_etag)
            || modified
                .as_ref()
                .is_some_and(|value| value != &response_modified)
        {
            return Err(mismatch(
                "The refreshed file changed between verification ranges.",
            ));
        }
        final_url = Some(response_url);
        if validator.is_none() {
            validator = Some(response_etag);
        }
        if modified.is_none() {
            modified = Some(response_modified);
        }
        let mut actual = Vec::with_capacity(expected.len());
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| JobError::new(ErrorKind::Network, "Refresh range was interrupted."))?
        {
            if actual.len() + chunk.len() > expected.len() {
                return Err(mismatch("The server exceeded the requested byte range."));
            }
            actual.extend_from_slice(&chunk);
        }
        if &actual != expected {
            return Err(mismatch(
                "The refreshed file does not match the saved partial.",
            ));
        }
    }
    Ok(VerifiedRefresh {
        final_url: final_url.expect("nonempty samples"),
        etag: validator.flatten(),
        last_modified: modified.flatten(),
        total,
    })
}

pub(crate) async fn verify_checkpoint(
    checkpoint: Checkpoint,
    ignore_modified: bool,
    candidate: RefreshCandidate,
) -> Result<VerifiedRefresh, JobError> {
    let segmented = checkpoint.segmented();
    let total = checkpoint
        .total
        .ok_or_else(|| mismatch("The saved download has no known total size."))?;
    if checkpoint.received() == 0 || checkpoint.received() >= total {
        return Err(mismatch(
            "The saved download has no unfinished bytes to resume.",
        ));
    }
    let samples = tokio::task::spawn_blocking(move || -> Result<Vec<(u64, Vec<u8>)>, JobError> {
        let mut file = output::inspect_checkpoint(&checkpoint, ignore_modified)
            .map_err(|_| mismatch("The saved partial changed or cannot be opened."))?;
        let mut samples: Vec<(u64, Vec<u8>)> = sample_ranges(checkpoint.durable)
            .into_iter()
            .map(|(start, end)| {
                file.seek(SeekFrom::Start(start))
                    .map_err(|_| mismatch("Cannot inspect saved bytes."))?;
                let mut bytes = vec![0; (end - start + 1) as usize];
                file.read_exact(&mut bytes)
                    .map_err(|_| mismatch("Cannot inspect saved bytes."))?;
                Ok((start, bytes))
            })
            .collect::<Result<_, JobError>>()?;
        for segment in &checkpoint.segments {
            let mut file = output::verify_segment(segment, ignore_modified)
                .map_err(|_| mismatch("A saved range changed or cannot be opened."))?;
            for (start, end) in sample_ranges(segment.saved) {
                file.seek(SeekFrom::Start(start))
                    .map_err(|_| mismatch("Cannot inspect saved range bytes."))?;
                let mut bytes = vec![0; (end - start + 1) as usize];
                file.read_exact(&mut bytes)
                    .map_err(|_| mismatch("Cannot inspect saved range bytes."))?;
                samples.push((segment.start + start, bytes));
            }
        }
        Ok(samples)
    })
    .await
    .map_err(|_| JobError::new(ErrorKind::Io, "Refresh verification worker stopped."))??;
    let client = client()?;
    let verified = verify_remote_samples(&client, &candidate, total, &samples).await?;
    if segmented && verified.etag.is_none() && verified.last_modified.is_none() {
        return Err(mismatch(
            "The refreshed server has no validator for saved ranges.",
        ));
    }
    Ok(verified)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusty_browser_bridge::protocol::RequestHeader;
    use std::{
        io::Write as _,
        net::{TcpListener, TcpStream},
        sync::{
            atomic::{AtomicBool, Ordering},
            Arc,
        },
        thread,
    };

    struct RangeServer {
        base: String,
        stop: Arc<AtomicBool>,
        thread: Option<thread::JoinHandle<()>>,
    }
    impl RangeServer {
        fn start() -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            listener.set_nonblocking(true).unwrap();
            let base = format!("http://{}", listener.local_addr().unwrap());
            let stop = Arc::new(AtomicBool::new(false));
            let flag = stop.clone();
            let thread = thread::spawn(move || {
                while !flag.load(Ordering::Acquire) {
                    match listener.accept() {
                        Ok((stream, _)) => answer(stream),
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            thread::sleep(Duration::from_millis(5))
                        }
                        Err(_) => break,
                    }
                }
            });
            Self {
                base,
                stop,
                thread: Some(thread),
            }
        }
        fn url(&self, path: &str) -> String {
            format!("{}{path}", self.base)
        }
    }
    impl Drop for RangeServer {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::Release);
            if let Some(thread) = self.thread.take() {
                thread.join().unwrap();
            }
        }
    }
    fn answer(mut stream: TcpStream) {
        let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
        let mut raw = Vec::new();
        let mut byte = [0u8; 1];
        while raw.len() < 8192 && stream.read(&mut byte).unwrap_or(0) == 1 {
            raw.push(byte[0]);
            if raw.ends_with(b"\r\n\r\n") {
                break;
            }
        }
        let request = String::from_utf8_lossy(&raw);
        let path = request.split_whitespace().nth(1).unwrap_or("/");
        if path == "/redirect" {
            let _ = stream.write_all(b"HTTP/1.1 302 Found\r\nLocation: /ok\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
            return;
        }
        if path == "/redirect-auth" {
            let _ = stream.write_all(b"HTTP/1.1 302 Found\r\nLocation: /no-cookie\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
            return;
        }
        if path == "/no-cookie" {
            let has_cookie = request
                .lines()
                .any(|line| line.to_ascii_lowercase().starts_with("cookie:"));
            let response = if has_cookie {
                b"HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                    .as_slice()
            } else {
                b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nOK".as_slice()
            };
            let _ = stream.write_all(response);
            return;
        }
        if path == "/auth"
            && !request
                .lines()
                .any(|line| line.eq_ignore_ascii_case("cookie: session=valid"))
        {
            let _ = stream.write_all(
                b"HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            );
            return;
        }
        let range = request
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                if !name.eq_ignore_ascii_case("range") {
                    return None;
                }
                let (start, end) = value.trim().strip_prefix("bytes=")?.split_once('-')?;
                Some((start.parse::<usize>().ok()?, end.parse::<usize>().ok()?))
            })
            .unwrap_or((0, 3));
        let mut body = *b"abcdefghijklmnop";
        if path == "/mismatch" {
            body[6] = b'X';
        }
        if path == "/ignored" {
            let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 16\r\nConnection: close\r\n\r\nabcdefghijklmnop");
            return;
        }
        let (start, end) = range;
        let total = if path == "/wrong-total" { 17 } else { 16 };
        let header_start = if path == "/wrong-range" {
            start + 1
        } else {
            start
        };
        let tag = if path == "/etag-change" && start > 0 {
            "\"changed\""
        } else {
            "\"fresh\""
        };
        let _ = write!(stream, "HTTP/1.1 206 Partial Content\r\nContent-Range: bytes {header_start}-{end}/{total}\r\nContent-Length: {}\r\n", end - start + 1);
        if path != "/missing-etag" {
            let _ = write!(stream, "ETag: {tag}\r\n");
        }
        if path == "/encoded" {
            let _ = stream.write_all(b"Content-Encoding: gzip\r\n");
        }
        let _ = stream.write_all(b"Connection: close\r\n\r\n");
        let _ = stream.write_all(&body[start..=end]);
    }

    fn candidate(url: String) -> RefreshCandidate {
        RefreshCandidate {
            url,
            source_page: None,
            request_context: Vec::new(),
        }
    }
    fn samples() -> Vec<(u64, Vec<u8>)> {
        vec![
            (0, b"abcd".to_vec()),
            (6, b"ghij".to_vec()),
            (12, b"mnop".to_vec()),
        ]
    }
    #[test]
    fn samples_cover_start_middle_and_end_without_exceeding_saved_bytes() {
        for received in [1, 1024, 65_536, 1_000_000] {
            let ranges = sample_ranges(received);
            assert_eq!(ranges[0].0, 0);
            assert_eq!(ranges.last().unwrap().1, received - 1);
            assert!(ranges
                .iter()
                .all(|(a, b)| a <= b && *b < received && *b - *a < 65_536));
        }
    }
    #[test]
    fn content_range_must_match_exact_sample_and_total() {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_RANGE, HeaderValue::from_static("bytes 20-29/100"));
        assert!(exact_range(&headers, 20, 29, 100, Some(10)));
        assert!(!exact_range(&headers, 20, 29, 101, Some(10)));
        assert!(!exact_range(&headers, 20, 29, 100, None));
    }

    #[tokio::test]
    async fn remote_samples_reject_changes_and_range_failures() {
        let server = RangeServer::start();
        let client = client().unwrap();
        let verified =
            verify_remote_samples(&client, &candidate(server.url("/ok")), 16, &samples())
                .await
                .unwrap();
        assert_eq!(verified.etag.as_deref(), Some("\"fresh\""));
        let without_tag = verify_remote_samples(
            &client,
            &candidate(server.url("/missing-etag")),
            16,
            &samples(),
        )
        .await
        .unwrap();
        assert_eq!(without_tag.etag, None);
        for path in [
            "/mismatch",
            "/wrong-total",
            "/wrong-range",
            "/ignored",
            "/encoded",
            "/etag-change",
            "/auth",
        ] {
            assert!(
                verify_remote_samples(&client, &candidate(server.url(path)), 16, &samples())
                    .await
                    .is_err(),
                "{path}"
            );
        }
    }

    #[tokio::test]
    async fn browser_headers_are_scoped_and_unobserved_redirects_fail() {
        let server = RangeServer::start();
        let client = client().unwrap();
        let mut protected = candidate(server.url("/auth"));
        protected.request_context.push(RequestHop {
            url: protected.url.clone(),
            headers: vec![RequestHeader {
                name: "Cookie".into(),
                value: "session=valid".into(),
            }],
        });
        assert!(verify_remote_samples(&client, &protected, 16, &samples())
            .await
            .is_ok());
        let mut redirected = candidate(server.url("/redirect"));
        redirected.request_context.push(RequestHop {
            url: redirected.url.clone(),
            headers: Vec::new(),
        });
        assert!(verify_remote_samples(&client, &redirected, 16, &samples())
            .await
            .is_err());
        assert!(verify_remote_samples(
            &client,
            &candidate(server.url("/redirect")),
            16,
            &samples()
        )
        .await
        .is_ok());
        protected.request_context[0].headers[0].value = "bad\r\nHost: evil".into();
        assert!(protected.validate().is_err());
    }

    #[tokio::test]
    async fn initial_get_uses_only_the_exact_observed_hop_headers() {
        let server = RangeServer::start();
        let client = client().unwrap();
        let first = server.url("/redirect-auth");
        let second = server.url("/no-cookie");
        let context = vec![
            RequestHop {
                url: first.clone(),
                headers: vec![RequestHeader {
                    name: "Cookie".into(),
                    value: "session=valid".into(),
                }],
            },
            RequestHop {
                url: second.clone(),
                headers: vec![],
            },
        ];
        let result = fetch_get(&client, &Url::parse(&second).unwrap(), &context)
            .await
            .unwrap();
        assert_eq!(result.status(), StatusCode::OK);
        assert_eq!(result.url().as_str(), second);
        let missing = fetch_get(&client, &Url::parse(&second).unwrap(), &context[..1]).await;
        assert!(missing.is_err());
    }

    #[tokio::test]
    async fn protected_chain_cannot_end_early_or_skip_an_observed_hop() {
        let server = RangeServer::start();
        let client = client().unwrap();
        let final_url = server.url("/no-cookie");
        let early = vec![
            RequestHop {
                url: server.url("/ignored"),
                headers: vec![],
            },
            RequestHop {
                url: final_url.clone(),
                headers: vec![],
            },
        ];
        assert!(fetch_get(&client, &Url::parse(&final_url).unwrap(), &early)
            .await
            .is_err());
        let skipped = vec![
            RequestHop {
                url: server.url("/redirect-auth"),
                headers: vec![],
            },
            RequestHop {
                url: server.url("/ignored"),
                headers: vec![],
            },
            RequestHop {
                url: final_url.clone(),
                headers: vec![],
            },
        ];
        assert!(
            fetch_get(&client, &Url::parse(&final_url).unwrap(), &skipped)
                .await
                .is_err()
        );
    }
}
