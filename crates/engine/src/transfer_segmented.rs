//! Coordinated byte-range transfers. Only the coordinator writes files or charges speed.
use super::{ProgressSink, TransferControl, TransferOutcome, TransferProgress};
use crate::{
    output::{self, OutputFile},
    refresh,
    storage::SegmentCheckpoint,
    ConnectionProgress, DownloadRequest, ErrorKind, JobError, SegmentProgress, SegmentState,
};
use reqwest::{
    header::{ACCEPT_ENCODING, CONTENT_ENCODING, CONTENT_RANGE, IF_RANGE, LAST_MODIFIED, RANGE},
    Client, Response, StatusCode,
};
use std::{collections::BTreeMap, io::SeekFrom, path::PathBuf, time::Duration};
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt},
    sync::{mpsc, watch},
    task::JoinSet,
};
use tokio_util::sync::CancellationToken;

const MIN_RANGE: u64 = 4 * 1024 * 1024;
const OVERLAP: u64 = 64 * 1024;
const MAX_SEGMENTS: usize = 128;

#[derive(Clone, PartialEq, Eq)]
enum Validator {
    Etag(String),
    Modified(String),
}
impl Validator {
    fn value(&self) -> &str {
        match self {
            Self::Etag(s) | Self::Modified(s) => s,
        }
    }
    fn etag(&self) -> Option<String> {
        match self {
            Self::Etag(s) => Some(s.clone()),
            _ => None,
        }
    }
    fn modified(&self) -> Option<String> {
        match self {
            Self::Modified(s) => Some(s.clone()),
            _ => None,
        }
    }
    fn matches(&self, response: &Response) -> bool {
        match self {
            Self::Etag(expected) => {
                refresh::strong_etag(response.headers()).as_ref() == Some(expected)
            }
            Self::Modified(expected) => {
                response
                    .headers()
                    .get(LAST_MODIFIED)
                    .and_then(|h| h.to_str().ok())
                    == Some(expected.as_str())
            }
        }
    }
}
#[derive(Clone)]
struct Probe {
    total: u64,
    final_url: String,
    validator: Validator,
    last_modified: Option<String>,
}
struct Span {
    telemetry: SegmentProgress,
    start: u64,
    end: u64,
    saved: u64,
    file: Option<File>,
    path: Option<PathBuf>,
}
impl Span {
    fn remaining(&self) -> u64 {
        (self.end - self.start + 1).saturating_sub(self.saved)
    }
    fn complete(&self) -> bool {
        self.saved == self.end - self.start + 1
    }
}
enum Event {
    Connected(usize),
    Chunk(usize, Vec<u8>),
    Done(usize),
    Ack(usize, u64, bool),
    Failed(usize, JobError),
}
fn mismatch(message: &str) -> JobError {
    JobError::new(ErrorKind::RestartRequired, message)
}
fn saved_validators_match(etag: Option<&str>, modified: Option<&str>, probe: &Probe) -> bool {
    etag.is_none_or(|tag| probe.validator.etag().as_deref() == Some(tag))
        && modified.is_none_or(|date| probe.last_modified.as_deref() == Some(date))
}
async fn emit(sender: &mpsc::Sender<Event>, stop: &CancellationToken, event: Event) -> bool {
    tokio::select! { biased;
        _ = stop.cancelled() => false,
        result = sender.send(event) => result.is_ok(),
    }
}

fn client(request: &DownloadRequest) -> Result<Client, JobError> {
    let redirect = if request.request_context.is_empty() {
        reqwest::redirect::Policy::limited(10)
    } else {
        reqwest::redirect::Policy::none()
    };
    Client::builder()
        .user_agent(request.options.user_agent.as_str())
        .redirect(redirect)
        .referer(false)
        .no_gzip()
        .no_brotli()
        .no_deflate()
        .no_zstd()
        .retry(reqwest::retry::never())
        .connect_timeout(Duration::from_secs(30))
        .build()
        .map_err(|_| JobError::new(ErrorKind::Network, "Cannot initialize the range client."))
}
async fn fetch(
    client: &Client,
    request: &DownloadRequest,
    start: u64,
    end: u64,
    if_range: Option<&str>,
) -> Result<Response, JobError> {
    if !request.request_context.is_empty() {
        return refresh::fetch_range(
            client,
            &request.url,
            &request.request_context,
            start,
            Some(end),
            if_range,
        )
        .await;
    }
    let mut builder = client
        .get(request.url.clone())
        .header(ACCEPT_ENCODING, "identity")
        .header(RANGE, format!("bytes={start}-{end}"));
    if let Some(value) = if_range {
        builder = builder.header(IF_RANGE, value);
    }
    if let Some(auth) = &request.authorization {
        builder = builder.basic_auth(&auth.login, Some(&auth.password));
    }
    let response = tokio::time::timeout(Duration::from_secs(30), builder.send())
        .await
        .map_err(|_| JobError::transient(ErrorKind::Network, "Range connection timed out."))?
        .map_err(|error| JobError::request(&error, "Range connection failed."))?;
    let error = JobError::http(
        response.status().as_u16(),
        format!(
            "Server returned HTTP {} for a range.",
            response.status().as_u16()
        ),
    );
    if error.retryable {
        return Err(error);
    }
    Ok(response)
}
fn identity(response: &Response) -> bool {
    response
        .headers()
        .get_all(CONTENT_ENCODING)
        .iter()
        .all(|h| h.to_str().is_ok_and(|s| s.eq_ignore_ascii_case("identity")))
}
async fn probe(client: &Client, request: &DownloadRequest) -> Result<Option<Probe>, JobError> {
    let mut response = fetch(client, request, 0, 0, None).await?;
    if !response.status().is_success() {
        let error = JobError::http(
            response.status().as_u16(),
            format!(
                "Server returned HTTP {} during range probing.",
                response.status().as_u16()
            ),
        );
        return if error.retryable {
            Err(error)
        } else {
            Ok(None)
        };
    }
    if response.status() != StatusCode::PARTIAL_CONTENT
        || !identity(&response)
        || response.content_length() != Some(1)
    {
        return Ok(None);
    }
    let total = response
        .headers()
        .get(CONTENT_RANGE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("bytes 0-0/"))
        .and_then(|value| value.parse::<u64>().ok());
    let Some(total) = total else {
        return Ok(None);
    };
    if total < 2 * MIN_RANGE {
        return Ok(None);
    }
    let last_modified = response
        .headers()
        .get(LAST_MODIFIED)
        .and_then(|h| h.to_str().ok())
        .filter(|s| !s.is_empty() && s.is_ascii() && !s.bytes().any(|b| b.is_ascii_control()))
        .map(str::to_owned);
    let validator = refresh::strong_etag(response.headers())
        .map(Validator::Etag)
        .or_else(|| last_modified.clone().map(Validator::Modified));
    let Some(validator) = validator else {
        return Ok(None);
    };
    let final_url = response.url().as_str().to_owned();
    let bytes = tokio::time::timeout(Duration::from_secs(30), response.chunk())
        .await
        .map_err(|_| JobError::transient(ErrorKind::Network, "Range probe timed out."))?
        .map_err(|_| JobError::transient(ErrorKind::Network, "Range probe was interrupted."))?
        .ok_or_else(|| {
            JobError::transient(ErrorKind::LengthMismatch, "Range probe ended early.")
        })?;
    let trailing = tokio::time::timeout(Duration::from_secs(30), response.chunk())
        .await
        .map_err(|_| JobError::transient(ErrorKind::Network, "Range probe timed out."))?
        .map_err(|_| JobError::transient(ErrorKind::Network, "Range probe was interrupted."))?;
    if bytes.len() != 1 || trailing.is_some() {
        return Ok(None);
    }
    Ok(Some(Probe {
        total,
        final_url,
        validator,
        last_modified,
    }))
}
fn valid_response(response: &Response, probe: &Probe, start: u64, end: u64) -> bool {
    response.status() == StatusCode::PARTIAL_CONTENT
        && identity(response)
        && response.url().as_str() == probe.final_url
        && probe.validator.matches(response)
        && refresh::exact_range(
            response.headers(),
            start,
            end,
            probe.total,
            response.content_length(),
        )
}
fn initial_ranges(total: u64, primary: u64, max: u8) -> Option<Vec<(u64, u64)>> {
    if max < 2 || total.saturating_sub(primary) < MIN_RANGE {
        return None;
    }
    let count = (total / MIN_RANGE).min(max as u64) as usize;
    if count < 2 {
        return None;
    }
    let first_end = (total / count as u64).max(primary).saturating_sub(1);
    let remaining = total - first_end - 1;
    let tails = (remaining / MIN_RANGE).min((count - 1) as u64) as usize;
    if tails == 0 {
        return None;
    }
    let mut ranges = vec![(0, first_end)];
    let mut start = first_end + 1;
    for n in 0..tails {
        let left = tails - n;
        let length = (total - start) / left as u64;
        let end = start + length - 1;
        ranges.push((start, end));
        start = end + 1;
    }
    if start != total || ranges.iter().any(|(a, b)| b - a + 1 < MIN_RANGE) {
        return None;
    }
    Some(ranges)
}

async fn checkpoints(spans: &[Span]) -> Result<Vec<SegmentCheckpoint>, JobError> {
    let mut result = Vec::with_capacity(spans.len().saturating_sub(1));
    for span in spans.iter().skip(1) {
        result.push(
            output::segment_checkpoint(
                span.file.as_ref().expect("sidecar"),
                span.path.as_ref().expect("sidecar path").clone(),
                span.start,
                span.end,
                span.saved,
            )
            .await?,
        );
    }
    Ok(result)
}
async fn save(
    output: &mut OutputFile,
    spans: &[Span],
    validator: &Validator,
    cleanup: Vec<PathBuf>,
) -> Result<(), JobError> {
    output
        .checkpoint_ranges(checkpoints(spans).await?, validator.modified(), cleanup)
        .await
}
fn received(primary: u64, spans: &[Span]) -> u64 {
    spans.iter().skip(1).fold(primary, |sum, s| {
        sum.saturating_add(s.saved.saturating_sub(primary.saturating_sub(s.start)))
    })
}
fn report(
    progress: &ProgressSink,
    primary: u64,
    spans: &[Span],
    total: u64,
    paused: bool,
    connection_limit: u8,
) {
    progress(TransferProgress {
        received: received(primary, spans),
        total: Some(total),
        resume_capable: true,
        paused,
        details: Some(ConnectionProgress {
            connection_limit,
            segments: spans
                .iter()
                .map(|span| {
                    let mut row = span.telemetry.clone();
                    row.start = span.start;
                    row.end = Some(span.end);
                    row.received = span.saved;
                    if span.complete()
                        && !row.state.is_active()
                        && row.state != SegmentState::Failed
                    {
                        row.state = SegmentState::Completed;
                    } else if paused && row.state != SegmentState::Completed {
                        row.state = SegmentState::Paused;
                    }
                    row
                })
                .collect(),
        }),
    });
}

async fn worker_inner(
    id: usize,
    client: Client,
    request: DownloadRequest,
    probe: Probe,
    start: u64,
    end: u64,
    mut endpoint: watch::Receiver<u64>,
    sender: mpsc::Sender<Event>,
    stop: CancellationToken,
) -> Result<(), JobError> {
    let mut response = tokio::select! { biased;
        _ = stop.cancelled() => return Ok(()),
        result = fetch(&client, &request, start, end, Some(probe.validator.value())) => result?,
    };
    if !response.status().is_success() {
        return Err(JobError::http(
            response.status().as_u16(),
            format!(
                "Server returned HTTP {} for a range.",
                response.status().as_u16()
            ),
        ));
    }
    if !valid_response(&response, &probe, start, end) {
        return Err(mismatch(
            "The server changed the file or returned an invalid range. Saved bytes were preserved.",
        ));
    }
    if !emit(&sender, &stop, Event::Connected(id)).await {
        return Ok(());
    }
    let mut position = start;
    let mut target = end;
    let mut acknowledged = false;
    loop {
        let proposed = *endpoint.borrow_and_update();
        if proposed != end && !acknowledged {
            let accepted = position <= proposed.saturating_add(1);
            if accepted {
                target = proposed;
            }
            acknowledged = true;
            if !emit(&sender, &stop, Event::Ack(id, proposed, accepted)).await {
                return Ok(());
            }
        }
        if position > target {
            let _ = emit(&sender, &stop, Event::Done(id)).await;
            return Ok(());
        }
        let chunk = tokio::select! { biased;
            _ = stop.cancelled() => return Ok(()),
            changed = endpoint.changed(), if !acknowledged => { if changed.is_err() { return Ok(()); } continue; },
            result = tokio::time::timeout(Duration::from_secs(30), response.chunk()) =>
                result.map_err(|_| JobError::transient(ErrorKind::Network, "A range made no progress for 30 seconds."))?
                    .map_err(|_| JobError::transient(ErrorKind::Network, "A range connection was interrupted."))?,
        };
        let Some(chunk) = chunk else {
            break;
        };
        if chunk.len() as u64 > end + 1 - position {
            return Err(mismatch("The server exceeded its declared range length."));
        }
        let take = chunk.len().min((target + 1 - position) as usize);
        if take > 0 && !emit(&sender, &stop, Event::Chunk(id, chunk[..take].to_vec())).await {
            return Ok(());
        }
        position += chunk.len() as u64;
    }
    if position != end + 1 {
        return Err(JobError::transient(
            ErrorKind::LengthMismatch,
            "A range ended before its declared length.",
        ));
    }
    let _ = emit(&sender, &stop, Event::Done(id)).await;
    Ok(())
}
fn launch(
    id: usize,
    spans: &mut [Span],
    client: &Client,
    request: &DownloadRequest,
    probe: &Probe,
    sender: &mpsc::Sender<Event>,
    stop: &CancellationToken,
    workers: &mut JoinSet<()>,
    active: &mut BTreeMap<usize, watch::Sender<u64>>,
) {
    let span = &mut spans[id];
    span.telemetry.generation += 1;
    span.telemetry.transferred = 0;
    span.telemetry.state = SegmentState::Connecting;
    let (end_sender, end_receiver) = watch::channel(span.end);
    active.insert(id, end_sender);
    let (client, request, probe, sender, stop) = (
        client.clone(),
        request.clone(),
        probe.clone(),
        sender.clone(),
        stop.clone(),
    );
    let start = span.start + span.saved;
    let end = span.end;
    workers.spawn(async move {
        let reporting_stop = stop.clone();
        if let Err(error) = worker_inner(
            id,
            client,
            request,
            probe,
            start,
            end,
            end_receiver,
            sender.clone(),
            stop,
        )
        .await
        {
            let _ = emit(&sender, &reporting_stop, Event::Failed(id, error)).await;
        }
    });
}
async fn quiesce(stop: &CancellationToken, workers: &mut JoinSet<()>) {
    stop.cancel();
    while workers.join_next().await.is_some() {}
}

pub(super) async fn run(
    request: &DownloadRequest,
    control: &TransferControl,
    progress: ProgressSink,
    output: &mut OutputFile,
) -> Result<Option<TransferOutcome>, JobError> {
    let saved = request.recovery.as_ref().and_then(|r| r.checkpoint());
    let previously_segmented = saved
        .as_ref()
        .is_some_and(|c| c.segmented() || !c.cleanup_pending.is_empty());
    if request.recovery.is_none() {
        return Ok(None);
    }
    if request.options.max_connections == 1 && !previously_segmented {
        return Ok(None);
    }
    progress(TransferProgress {
        received: saved.as_ref().map_or(0, |c| c.received()),
        total: saved.as_ref().and_then(|c| c.total),
        resume_capable: saved.as_ref().is_some_and(|c| c.durable > 0),
        paused: false,
        details: Some(ConnectionProgress {
            connection_limit: request.options.max_connections,
            segments: vec![],
        }),
    });
    let client = client(request)?;
    let probed = loop {
        if control.token.is_cancelled() {
            return Ok(Some(TransferOutcome::Cancelled));
        }
        if control.is_paused() {
            progress(TransferProgress {
                received: saved.as_ref().map_or(0, |checkpoint| checkpoint.received()),
                total: saved.as_ref().and_then(|checkpoint| checkpoint.total),
                resume_capable: saved
                    .as_ref()
                    .is_some_and(|checkpoint| checkpoint.durable > 0),
                paused: true,
                details: None,
            });
            if !control.wait_until_resumed_output(output).await? {
                return Ok(Some(TransferOutcome::Cancelled));
            }
        }
        tokio::select! { biased;
            _ = control.token.cancelled() => return Ok(Some(TransferOutcome::Cancelled)),
            _ = control.pause_requested() => continue,
            result = probe(&client, request) => break result?,
        }
    };
    let Some(probe) = probed else {
        return if previously_segmented {
            Err(mismatch(
                "The server no longer supports verified byte ranges. Saved ranges were preserved.",
            ))
        } else {
            Ok(None)
        };
    };
    if let Some(c) = &saved {
        if c.received() > 0
            && (c.total != Some(probe.total)
                || c.response_url.as_deref() != Some(probe.final_url.as_str())
                || !saved_validators_match(c.etag.as_deref(), c.last_modified.as_deref(), &probe))
        {
            return if previously_segmented {
                Err(mismatch(
                    "The saved ranges belong to a different response. They were preserved.",
                ))
            } else {
                Ok(None)
            };
        }
    }
    let primary = output.length().await?;
    let mut spans = Vec::new();
    if previously_segmented {
        let c = saved.as_ref().expect("saved range map");
        let mut segments = c.segments.clone();
        segments.sort_by_key(|s| s.start);
        let first_end = segments.first().map_or(probe.total - 1, |s| s.start - 1);
        spans.push(Span {
            telemetry: SegmentProgress {
                id: spans.len() as u32,
                start: 0,
                end: None,
                received: 0,
                transferred: 0,
                generation: 0,
                state: SegmentState::Waiting,
            },
            start: 0,
            end: first_end,
            saved: primary.min(first_end + 1),
            file: None,
            path: None,
        });
        let mut next = first_end + 1;
        for s in &segments {
            if s.start != next || s.end >= probe.total {
                return Err(mismatch("The saved range map has a gap or overlap."));
            }
            let file = match output::verify_segment(s, request.options.ignore_modified_time) {
                Ok(file) => Some(File::from_std(file)),
                Err(e)
                    if c.cleanup_pending.contains(&s.partial)
                        && primary == probe.total
                        && e.kind() == std::io::ErrorKind::NotFound =>
                {
                    None
                }
                Err(_) => {
                    return Err(mismatch(
                        "A saved range file changed. All bytes were preserved.",
                    ))
                }
            };
            spans.push(Span {
                telemetry: SegmentProgress {
                    id: spans.len() as u32,
                    start: 0,
                    end: None,
                    received: 0,
                    transferred: 0,
                    generation: 0,
                    state: SegmentState::Waiting,
                },
                start: s.start,
                end: s.end,
                saved: s.saved,
                file,
                path: Some(s.partial.clone()),
            });
            next = s.end + 1;
        }
        if next != probe.total {
            return Err(mismatch("The saved range map is incomplete."));
        }
    } else {
        let Some(ranges) = initial_ranges(probe.total, primary, request.options.max_connections)
        else {
            return Ok(None);
        };
        let (start, end) = ranges[0];
        spans.push(Span {
            telemetry: SegmentProgress {
                id: spans.len() as u32,
                start: 0,
                end: None,
                received: 0,
                transferred: 0,
                generation: 0,
                state: SegmentState::Waiting,
            },
            start,
            end,
            saved: primary,
            file: None,
            path: None,
        });
        for (start, end) in ranges.into_iter().skip(1) {
            let (file, segment) =
                output::reserve_segment(output.partial_path(), start, end).await?;
            spans.push(Span {
                telemetry: SegmentProgress {
                    id: spans.len() as u32,
                    start: 0,
                    end: None,
                    received: 0,
                    transferred: 0,
                    generation: 0,
                    state: SegmentState::Waiting,
                },
                start,
                end,
                saved: 0,
                file: Some(file),
                path: Some(segment.partial),
            });
        }
    }
    if spans.len() > MAX_SEGMENTS {
        return Err(mismatch("Too many saved ranges."));
    }
    output.representation(
        Some(probe.total),
        Some(probe.final_url.clone()),
        probe.validator.etag(),
    );
    report(
        &progress,
        primary,
        &spans,
        probe.total,
        false,
        request.options.max_connections,
    );
    if primary == probe.total && previously_segmented {
        if matches!(&probe.validator, Validator::Modified(_)) {
            for (start, expected) in output.refresh_samples(primary).await? {
                let end = start + expected.len() as u64 - 1;
                let mut response =
                    fetch(&client, request, start, end, Some(probe.validator.value())).await?;
                if !valid_response(&response, &probe, start, end) {
                    return Err(mismatch("The assembled file changed on the server."));
                }
                let mut actual = Vec::with_capacity(expected.len());
                while let Some(chunk) = response
                    .chunk()
                    .await
                    .map_err(|_| mismatch("Assembled-file recheck was interrupted."))?
                {
                    if actual.len() + chunk.len() > expected.len() {
                        return Err(mismatch("Assembled-file recheck exceeded its range."));
                    }
                    actual.extend_from_slice(&chunk);
                }
                if actual != expected {
                    return Err(mismatch("The assembled file differs from the server."));
                }
            }
        }
        let checkpoint = saved.as_ref().expect("saved range map");
        let cleanup: Vec<PathBuf> = checkpoint
            .segments
            .iter()
            .map(|s| s.partial.clone())
            .collect();
        output
            .checkpoint_ranges(
                checkpoint.segments.clone(),
                probe.validator.modified(),
                cleanup,
            )
            .await?;
        for span in spans.iter_mut().skip(1) {
            if let Some(file) = span.file.take() {
                output::remove_segment(file, span.path.as_ref().expect("sidecar path").clone())
                    .await?;
            }
        }
        output.checkpoint_ranges(vec![], None, vec![]).await?;
        return Ok(Some(TransferOutcome::Completed { bytes: probe.total }));
    }
    if saved
        .as_ref()
        .is_some_and(|c| c.received() > 0 && (c.refresh_pending || c.etag.is_none()))
    {
        if !previously_segmented {
            // Record newly reserved sidecars before a recheck can pause or exit.
            save(output, &spans, &probe.validator, vec![]).await?;
        }
        if !verify_saved_controlled(
            &client, request, &probe, output, &mut spans, control, &progress,
        )
        .await?
        {
            return Ok(Some(TransferOutcome::Cancelled));
        }
        output.clear_refresh_pending();
    }
    // Verification is read-only. Discard only verified, uncheckpointed tails
    // when preparing to resume writes; the following checkpoint syncs them.
    for span in spans.iter_mut().skip(1) {
        let file = span.file.as_mut().expect("sidecar");
        let length = file
            .metadata()
            .await
            .map_err(|_| JobError::new(ErrorKind::Io, "Cannot inspect saved range length."))?
            .len();
        if length > span.saved {
            file.set_len(span.saved).await.map_err(|_| {
                JobError::new(ErrorKind::Io, "Cannot trim uncheckpointed range bytes.")
            })?;
        }
    }
    save(output, &spans, &probe.validator, vec![]).await?;
    for span in spans.iter_mut().skip(1) {
        span.file
            .as_mut()
            .expect("sidecar")
            .seek(SeekFrom::Start(span.saved))
            .await
            .map_err(|_| JobError::new(ErrorKind::Io, "Cannot position saved range bytes."))?;
    }
    let mut primary_len = primary;
    report(
        &progress,
        primary_len,
        &spans,
        probe.total,
        false,
        request.options.max_connections,
    );
    while spans.iter().any(|s| !s.complete()) {
        if control.token.is_cancelled() {
            save(output, &spans, &probe.validator, vec![]).await?;
            return Ok(Some(TransferOutcome::Cancelled));
        }
        if control.is_paused() {
            save(output, &spans, &probe.validator, vec![]).await?;
            report(
                &progress,
                primary_len,
                &spans,
                probe.total,
                true,
                request.options.max_connections,
            );
            if !control.wait_until_resumed_output(output).await? {
                return Ok(Some(TransferOutcome::Cancelled));
            }
        }
        let stop = control.token.child_token();
        let (sender, mut receiver) = mpsc::channel(request.options.max_connections as usize * 4);
        let mut workers = JoinSet::new();
        let mut active = BTreeMap::new();
        for id in 0..spans.len() {
            if !spans[id].complete() && active.len() < request.options.max_connections as usize {
                launch(
                    id,
                    &mut spans,
                    &client,
                    request,
                    &probe,
                    &sender,
                    &stop,
                    &mut workers,
                    &mut active,
                );
            }
        }
        report(
            &progress,
            primary_len,
            &spans,
            probe.total,
            false,
            request.options.max_connections,
        );
        let mut pending_split: Option<(usize, u64, u64)> = None;
        let mut checkpoint_at = tokio::time::Instant::now();
        loop {
            if active.is_empty() {
                break;
            }
            tokio::select! { biased;
                _ = control.token.cancelled() => {
                    quiesce(&stop, &mut workers).await;
                    save(output, &spans, &probe.validator, vec![]).await?;
                    return Ok(Some(TransferOutcome::Cancelled));
                }
                _ = control.pause_requested() => {
                    quiesce(&stop, &mut workers).await;
                    for span in &mut spans {
                        if span.telemetry.state.is_active() { span.telemetry.state = SegmentState::Paused; }
                    }
                    save(output, &spans, &probe.validator, vec![]).await?;
                    report(&progress, primary_len, &spans, probe.total, true, request.options.max_connections);
                    if !control.wait_until_resumed_output(output).await? { return Ok(Some(TransferOutcome::Cancelled)); }
                    break;
                }
                _ = tokio::time::sleep_until(checkpoint_at + Duration::from_secs(5)) => {
                    if output.adopt_recovery(control.recovery()) { /* included in the same range checkpoint */ }
                    let result = save(output, &spans, &probe.validator, vec![]).await;
                    if control.recovery().is_some() { control.checkpoint_ack.send_replace(Some(result.clone())); }
                    result?;
                    checkpoint_at = tokio::time::Instant::now();
                }
                joined = workers.join_next(), if !workers.is_empty() => {
                    if joined.is_some_and(|r| r.is_err()) {
                        quiesce(&stop, &mut workers).await;
                        save(output, &spans, &probe.validator, vec![]).await?;
                        return Err(JobError::new(ErrorKind::Network, "A range worker stopped unexpectedly."));
                    }
                }
                event = receiver.recv() => match event {
                    Some(Event::Connected(id)) => {
                        spans[id].telemetry.state = SegmentState::Receiving;
                        report(&progress, primary_len, &spans, probe.total, false, request.options.max_connections);
                    }
                    Some(Event::Chunk(id, bytes)) => {
                        if spans[id].saved.checked_add(bytes.len() as u64).is_none_or(|n| n > spans[id].end - spans[id].start + 1) {
                            quiesce(&stop, &mut workers).await;
                            return Err(mismatch("A range exceeded its allocated endpoint."));
                        }
                        let mut at = 0;
                        while at < bytes.len() {
                            let limit = control.settings.borrow().bytes_per_second;
                            let quantum = if limit == 0 { 16_384 } else { (limit / 10).clamp(1,16_384) as usize };
                            let end = (at + quantum).min(bytes.len());
                            if !control.pace(end - at).await { break; }
                            if id == 0 { output.write(&bytes[at..end]).await?; primary_len += (end-at) as u64; }
                            else { spans[id].file.as_mut().expect("sidecar").write_all(&bytes[at..end]).await
                                .map_err(|_| JobError::new(ErrorKind::Io, "Writing a range file failed."))?; }
                            spans[id].saved += (end-at) as u64;
                            spans[id].telemetry.transferred = spans[id].telemetry.transferred.saturating_add((end-at) as u64);
                            at = end;
                            report(&progress, primary_len, &spans, probe.total, false, request.options.max_connections);
                        }
                    }
                    Some(Event::Failed(id, error)) => {
                        quiesce(&stop, &mut workers).await;
                        for (index, span) in spans.iter_mut().enumerate() {
                            if index == id { span.telemetry.state = SegmentState::Failed; }
                            else if span.telemetry.state.is_active() { span.telemetry.state = SegmentState::Cancelled; }
                        }
                        report(&progress, primary_len, &spans, probe.total, false, request.options.max_connections);
                        save(output, &spans, &probe.validator, vec![]).await?;
                        return Err(error);
                    }
                    Some(Event::Done(id)) => {
                        active.remove(&id);
                        if let Some((waiting, _, _)) = pending_split {
                            if waiting == id { pending_split = None; }
                        }
                        if !spans[id].complete() {
                            spans[id].telemetry.state = SegmentState::Failed;
                            report(&progress, primary_len, &spans, probe.total, false, request.options.max_connections);
                            quiesce(&stop, &mut workers).await;
                            save(output, &spans, &probe.validator, vec![]).await?;
                            return Err(JobError::new(ErrorKind::LengthMismatch, "A range ended before all bytes were saved."));
                        }
                        spans[id].telemetry.state = SegmentState::Completed;
                        report(&progress, primary_len, &spans, probe.total, false, request.options.max_connections);
                        if pending_split.is_none() && spans.len() < MAX_SEGMENTS
                            && active.len() < request.options.max_connections as usize {
                            let candidate = active.keys().copied().max_by_key(|i| spans[*i].remaining());
                            if let Some(candidate) = candidate.filter(|i| spans[*i].remaining() >= 2 * MIN_RANGE) {
                                let current = spans[candidate].start + spans[candidate].saved;
                                let old_end = spans[candidate].end;
                                let split_end = current + (old_end + 1 - current) / 2 - 1;
                                if let Some(endpoint) = active.get(&candidate) {
                                    endpoint.send_replace(split_end);
                                    pending_split = Some((candidate, split_end, old_end));
                                }
                            }
                        }
                    }
                    Some(Event::Ack(id, proposed, accepted)) => {
                        if let Some((candidate, split_end, old_end)) = pending_split.take() {
                            if candidate == id && proposed == split_end && accepted {
                                spans[id].end = split_end;
                                let (file, segment) = output::reserve_segment(output.partial_path(), split_end + 1, old_end).await?;
                                spans.push(Span {
                telemetry: SegmentProgress { id: spans.len() as u32, start: 0, end: None, received: 0,
                    transferred: 0, generation: 0, state: SegmentState::Waiting }, start: split_end + 1, end: old_end, saved: 0,
                                    file: Some(file), path: Some(segment.partial) });
                                save(output, &spans, &probe.validator, vec![]).await?;
                                let next = spans.len() - 1;
                                launch(next, &mut spans, &client, request, &probe, &sender, &stop, &mut workers, &mut active);
                                report(&progress, primary_len, &spans, probe.total, false, request.options.max_connections);
                            }
                        }
                    }
                    None => { quiesce(&stop, &mut workers).await; return Err(JobError::new(ErrorKind::Network, "Range workers stopped unexpectedly.")); }
                }
            }
        }
        quiesce(&stop, &mut workers).await;
        save(output, &spans, &probe.validator, vec![]).await?;
        for span in spans.iter_mut().skip(1) {
            span.file
                .as_mut()
                .expect("sidecar")
                .seek(SeekFrom::Start(span.saved))
                .await
                .map_err(|_| JobError::new(ErrorKind::Io, "Cannot position saved range bytes."))?;
        }
    }
    spans.sort_by_key(|s| s.start);
    if matches!(&probe.validator, Validator::Modified(_)) {
        if !verify_saved_controlled(
            &client, request, &probe, output, &mut spans, control, &progress,
        )
        .await?
        {
            return Ok(Some(TransferOutcome::Cancelled));
        }
    }
    assemble(
        output,
        &mut spans,
        control,
        &probe,
        &progress,
        primary_len,
        request.options.max_connections,
    )
    .await
    .map(Some)
}

async fn verify_saved_controlled(
    client: &Client,
    request: &DownloadRequest,
    probe: &Probe,
    output: &mut OutputFile,
    spans: &mut [Span],
    control: &TransferControl,
    progress: &ProgressSink,
) -> Result<bool, JobError> {
    // Both callers have checkpointed their current ranges before this read-only
    // recheck. Preserve that map: pre-write sidecars may still have saved tails.
    loop {
        if control.token.is_cancelled() {
            output.checkpoint().await?;
            return Ok(false);
        }
        if control.is_paused() {
            output.checkpoint().await?;
            report(
                progress,
                output.length().await?,
                spans,
                probe.total,
                true,
                request.options.max_connections,
            );
            if !control.wait_until_resumed_output(output).await? {
                return Ok(false);
            }
        }
        tokio::select! { biased;
            _ = control.token.cancelled() => {
                output.checkpoint().await?;
                return Ok(false);
            }
            _ = control.pause_requested() => continue,
            result = verify_saved(client, request, probe, output, spans) => {
                result?;
                return Ok(true);
            }
        }
    }
}

async fn verify_saved(
    client: &Client,
    request: &DownloadRequest,
    probe: &Probe,
    output: &mut OutputFile,
    spans: &mut [Span],
) -> Result<(), JobError> {
    let primary = output.length().await?;
    for id in 0..spans.len() {
        let span = &mut spans[id];
        let saved = if id == 0 {
            primary.min(span.end + 1)
        } else {
            span.saved
        };
        if saved == 0 {
            continue;
        }
        let mut starts = vec![
            0,
            saved.saturating_sub(OVERLAP) / 2,
            saved.saturating_sub(OVERLAP),
        ];
        starts.sort_unstable();
        starts.dedup();
        for offset in starts {
            let size = (saved - offset).min(OVERLAP) as usize;
            let expected = if id == 0 {
                let mut file = File::open(output.partial_path())
                    .await
                    .map_err(|_| mismatch("Cannot inspect saved primary bytes."))?;
                file.seek(SeekFrom::Start(offset))
                    .await
                    .map_err(|_| mismatch("Cannot inspect saved primary bytes."))?;
                let mut bytes = vec![0; size];
                file.read_exact(&mut bytes)
                    .await
                    .map_err(|_| mismatch("Cannot inspect saved primary bytes."))?;
                bytes
            } else {
                let file = span.file.as_mut().expect("saved sidecar");
                file.seek(SeekFrom::Start(offset))
                    .await
                    .map_err(|_| mismatch("Cannot inspect saved range bytes."))?;
                let mut bytes = vec![0; size];
                file.read_exact(&mut bytes)
                    .await
                    .map_err(|_| mismatch("Cannot inspect saved range bytes."))?;
                bytes
            };
            let start = span.start + offset;
            let end = start + size as u64 - 1;
            let mut response =
                fetch(client, request, start, end, Some(probe.validator.value())).await?;
            if !valid_response(&response, probe, start, end) {
                return Err(mismatch("Saved-byte recheck returned a changed range."));
            }
            let mut actual = Vec::with_capacity(size);
            while let Some(chunk) = tokio::time::timeout(Duration::from_secs(30), response.chunk())
                .await
                .map_err(|_| {
                    JobError::transient(ErrorKind::Network, "Saved-byte recheck timed out.")
                })?
                .map_err(|_| {
                    JobError::transient(ErrorKind::Network, "Saved-byte recheck was interrupted.")
                })?
            {
                if actual.len() + chunk.len() > size {
                    return Err(mismatch("Saved-byte recheck exceeded its range."));
                }
                actual.extend_from_slice(&chunk);
            }
            if actual.len() < size {
                return Err(JobError::transient(
                    ErrorKind::LengthMismatch,
                    "Saved-byte recheck ended early.",
                ));
            }
            if actual != expected {
                return Err(mismatch(
                    "Adjacent range overlap or saved bytes differ from the server.",
                ));
            }
        }
    }
    for boundary in 1..spans.len() {
        let (before, after) = spans.split_at_mut(boundary);
        let left = &mut before[boundary - 1];
        let right = &mut after[0];
        if !left.complete() || right.saved == 0 || left.end + 1 != right.start {
            continue;
        }
        let width = OVERLAP.min(left.saved).min(right.saved) as usize;
        let mut expected = Vec::with_capacity(width * 2);
        if left.start == 0 {
            let mut file = File::open(output.partial_path())
                .await
                .map_err(|_| mismatch("Cannot inspect adjacent saved bytes."))?;
            file.seek(SeekFrom::Start(left.end + 1 - width as u64))
                .await
                .map_err(|_| mismatch("Cannot inspect adjacent saved bytes."))?;
            let mut bytes = vec![0; width];
            file.read_exact(&mut bytes)
                .await
                .map_err(|_| mismatch("Cannot inspect adjacent saved bytes."))?;
            expected.extend_from_slice(&bytes);
        } else {
            let file = left.file.as_mut().expect("saved sidecar");
            file.seek(SeekFrom::Start(left.saved - width as u64))
                .await
                .map_err(|_| mismatch("Cannot inspect adjacent saved bytes."))?;
            let mut bytes = vec![0; width];
            file.read_exact(&mut bytes)
                .await
                .map_err(|_| mismatch("Cannot inspect adjacent saved bytes."))?;
            expected.extend_from_slice(&bytes);
        }
        let file = right.file.as_mut().expect("saved sidecar");
        file.seek(SeekFrom::Start(0))
            .await
            .map_err(|_| mismatch("Cannot inspect adjacent saved bytes."))?;
        let mut bytes = vec![0; width];
        file.read_exact(&mut bytes)
            .await
            .map_err(|_| mismatch("Cannot inspect adjacent saved bytes."))?;
        expected.extend_from_slice(&bytes);
        let start = right.start - width as u64;
        let end = right.start + width as u64 - 1;
        let mut response =
            fetch(client, request, start, end, Some(probe.validator.value())).await?;
        if !valid_response(&response, probe, start, end) {
            return Err(mismatch(
                "Adjacent overlap recheck returned a changed range.",
            ));
        }
        let mut actual = Vec::with_capacity(width * 2);
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| mismatch("Adjacent overlap recheck was interrupted."))?
        {
            if actual.len() + chunk.len() > expected.len() {
                return Err(mismatch("Adjacent overlap recheck exceeded its range."));
            }
            actual.extend_from_slice(&chunk);
        }
        if actual != expected {
            return Err(mismatch(
                "Adjacent saved ranges do not match one server response.",
            ));
        }
    }
    Ok(())
}

async fn assemble(
    output: &mut OutputFile,
    spans: &mut [Span],
    control: &TransferControl,
    probe: &Probe,
    progress: &ProgressSink,
    mut primary: u64,
    connection_limit: u8,
) -> Result<TransferOutcome, JobError> {
    let cleanup: Vec<PathBuf> = spans
        .iter()
        .skip(1)
        .filter_map(|s| s.path.clone())
        .collect();
    for index in 1..spans.len() {
        if control.token.is_cancelled() {
            return Ok(TransferOutcome::Cancelled);
        }
        let start = spans[index].start;
        if primary < start || primary > spans[index].end + 1 {
            return Err(mismatch("The assembled partial has a gap or overlap."));
        }
        let skip = primary - start;
        if skip < spans[index].saved {
            output.seek_end().await?;
            let mut left = spans[index].saved - skip;
            let mut buffer = [0u8; 65_536];
            while left > 0 {
                if control.token.is_cancelled() {
                    return Ok(TransferOutcome::Cancelled);
                }
                if control.is_paused() {
                    save(output, spans, &probe.validator, cleanup.clone()).await?;
                    report(
                        progress,
                        primary,
                        spans,
                        probe.total,
                        true,
                        connection_limit,
                    );
                    if !control.wait_until_resumed_output(output).await? {
                        return Ok(TransferOutcome::Cancelled);
                    }
                }
                let size = left.min(buffer.len() as u64) as usize;
                let file = spans[index]
                    .file
                    .as_mut()
                    .ok_or_else(|| mismatch("A required range file is missing."))?;
                file.seek(SeekFrom::Start(primary - start))
                    .await
                    .map_err(|_| JobError::new(ErrorKind::Io, "Cannot seek saved range bytes."))?;
                file.read_exact(&mut buffer[..size])
                    .await
                    .map_err(|_| JobError::new(ErrorKind::Io, "Cannot read saved range bytes."))?;
                output.write(&buffer[..size]).await?;
                primary += size as u64;
                left -= size as u64;
            }
            save(output, spans, &probe.validator, cleanup.clone()).await?;
            report(
                progress,
                primary,
                spans,
                probe.total,
                false,
                connection_limit,
            );
        }
    }
    if primary != probe.total {
        return Err(mismatch("The assembled partial is incomplete."));
    }
    save(output, spans, &probe.validator, cleanup).await?;
    for span in spans.iter_mut().skip(1) {
        if control.token.is_cancelled() {
            return Ok(TransferOutcome::Cancelled);
        }
        if control.is_paused() {
            if !control.wait_until_resumed_output(output).await? {
                return Ok(TransferOutcome::Cancelled);
            }
        }
        if let Some(file) = span.file.take() {
            output::remove_segment(file, span.path.as_ref().expect("sidecar path").clone()).await?;
        }
    }
    output.checkpoint_ranges(vec![], None, vec![]).await?;
    Ok(TransferOutcome::Completed { bytes: probe.total })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn refreshed_checkpoint_accepts_both_stable_validators() {
        let probe = Probe {
            total: 8 * MIN_RANGE,
            final_url: "https://example.test/file".into(),
            validator: Validator::Etag("\"v1\"".into()),
            last_modified: Some("Wed, 21 Oct 2026 00:00:00 GMT".into()),
        };
        assert!(saved_validators_match(
            Some("\"v1\""),
            Some("Wed, 21 Oct 2026 00:00:00 GMT"),
            &probe,
        ));
        assert!(!saved_validators_match(
            Some("\"v1\""),
            Some("Tue, 20 Oct 2026 00:00:00 GMT"),
            &probe,
        ));
    }
    #[test]
    fn ranges_are_contiguous_and_never_smaller_than_four_mib() {
        let ranges = initial_ranges(80 * MIN_RANGE, 0, 8).unwrap();
        assert_eq!(ranges.len(), 8);
        assert_eq!(ranges.first().unwrap().0, 0);
        assert_eq!(ranges.last().unwrap().1, 80 * MIN_RANGE - 1);
        for pair in ranges.windows(2) {
            assert_eq!(pair[0].1 + 1, pair[1].0);
        }
        assert!(ranges.iter().all(|(a, b)| b - a + 1 >= MIN_RANGE));
        assert!(initial_ranges(2 * MIN_RANGE - 1, 0, 8).is_none());
    }
    #[test]
    fn verified_old_prefix_can_be_partitioned() {
        let prefix = 17 * MIN_RANGE;
        let ranges = initial_ranges(80 * MIN_RANGE, prefix, 8).unwrap();
        assert!(ranges[0].1 + 1 >= prefix);
        assert_eq!(ranges[1].0, ranges[0].1 + 1);
    }
}
