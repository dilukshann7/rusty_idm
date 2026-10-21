use crate::{
    output::OutputFile,
    refresh::{self, RefreshCandidate},
    DownloadRequest, ErrorKind, JobError,
};
use reqwest::header::{HeaderMap, CONTENT_RANGE, ETAG};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;
#[path = "transfer_segmented.rs"]
mod segmented;

#[derive(Clone, Copy, Debug)]
pub struct TransferProgress {
    pub received: u64,
    pub total: Option<u64>,
    pub resume_capable: bool,
    pub paused: bool,
}
pub type ProgressSink = Arc<dyn Fn(TransferProgress) + Send + Sync>;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransferOutcome {
    Completed { bytes: u64 },
    Cancelled,
}

#[derive(Clone)]
pub struct TransferControl {
    token: CancellationToken,
    commit: Arc<Mutex<CommitState>>,
    settings: watch::Sender<TransferSettings>,
    preserve: Arc<std::sync::atomic::AtomicBool>,
    recovery: Arc<Mutex<Option<crate::storage::Recovery>>>,
    confirmed: Arc<std::sync::atomic::AtomicBool>,
    checkpoint_ack: watch::Sender<Option<Result<(), JobError>>>,
}
#[derive(Clone, Copy, Default)]
struct TransferSettings {
    paused: bool,
    bytes_per_second: u64,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum CommitState {
    Pending,
    Cancelled,
    Committed,
}
impl TransferControl {
    pub fn new(token: CancellationToken) -> Self {
        Self {
            token,
            commit: Arc::new(Mutex::new(CommitState::Pending)),
            settings: watch::channel(TransferSettings::default()).0,
            preserve: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            recovery: Arc::new(Mutex::new(None)),
            confirmed: Arc::new(std::sync::atomic::AtomicBool::new(true)),
            checkpoint_ack: watch::channel(None).0,
        }
    }
    pub(crate) fn is_paused(&self) -> bool {
        self.settings.borrow().paused
    }
    pub(crate) fn require_confirmation(&self, required: bool) {
        self.confirmed
            .store(!required, std::sync::atomic::Ordering::Release);
    }
    pub(crate) fn confirm(&self) {
        self.confirmed
            .store(true, std::sync::atomic::Ordering::Release);
        self.settings.send_modify(|_| {});
    }
    pub(crate) async fn await_confirmation(
        &self,
        output: &mut OutputFile,
        progress: &ProgressSink,
        bytes: u64,
    ) -> Result<bool, JobError> {
        let mut settings = self.settings.subscribe();
        if self.confirmed.load(std::sync::atomic::Ordering::Acquire) {
            return Ok(true);
        }
        self.try_pause();
        output.checkpoint().await?;
        progress(TransferProgress {
            received: bytes,
            total: Some(bytes),
            resume_capable: true,
            paused: true,
        });
        loop {
            if self.token.is_cancelled() {
                return Ok(false);
            }
            if self.confirmed.load(std::sync::atomic::Ordering::Acquire)
                && !settings.borrow_and_update().paused
            {
                return Ok(true);
            }
            tokio::select! {biased;_=self.token.cancelled()=>return Ok(false),_=settings.changed()=>{}}
        }
    }
    pub(crate) fn attach_recovery(&self, recovery: Option<crate::storage::Recovery>) {
        *self.recovery.lock().unwrap_or_else(|p| p.into_inner()) = recovery;
        self.checkpoint_ack.send_replace(None);
        self.settings.send_modify(|_| {});
    }
    pub(crate) fn recovery(&self) -> Option<crate::storage::Recovery> {
        self.recovery
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clone()
    }
    pub(crate) async fn wait_checkpoint(&self) -> Result<(), JobError> {
        let mut ack = self.checkpoint_ack.subscribe();
        tokio::time::timeout(Duration::from_secs(10),async {
            loop {
                if let Some(result)=ack.borrow_and_update().clone(){return result;}
                ack.changed().await.map_err(|_|JobError::new(ErrorKind::Storage,"Queue storage failed: checkpoint acknowledgement was lost."))?;
            }
        }).await.map_err(|_|JobError::new(ErrorKind::Storage,"Queue storage failed: checkpoint acknowledgement timed out; the entry remains held."))?
    }
    pub(crate) fn preserve_on_exit(&self) {
        self.preserve
            .store(true, std::sync::atomic::Ordering::Release);
        self.try_cancel();
    }
    pub(crate) fn preserving(&self) -> bool {
        self.preserve.load(std::sync::atomic::Ordering::Acquire)
    }
    pub fn token(&self) -> CancellationToken {
        self.token.clone()
    }
    pub fn try_pause(&self) -> bool {
        self.set_paused(true)
    }
    pub fn try_resume(&self) -> bool {
        self.set_paused(false)
    }
    fn set_paused(&self, paused: bool) -> bool {
        let state = self.commit.lock().unwrap_or_else(|p| p.into_inner());
        if *state != CommitState::Pending || self.token.is_cancelled() {
            return false;
        }
        self.settings
            .send_modify(|settings| settings.paused = paused);
        true
    }
    pub fn set_speed_limit(&self, bytes_per_second: Option<u64>) {
        self.settings
            .send_modify(|settings| settings.bytes_per_second = bytes_per_second.unwrap_or(0));
    }
    pub(crate) async fn wait_until_resumed_output(
        &self,
        output: &mut OutputFile,
    ) -> Result<bool, JobError> {
        let mut settings = self.settings.subscribe();
        loop {
            if output.adopt_recovery(self.recovery()) {
                let result = output.checkpoint().await;
                self.checkpoint_ack.send_replace(Some(result.clone()));
                result?;
            }
            if self.token.is_cancelled() {
                return Ok(false);
            }
            if !settings.borrow_and_update().paused {
                return Ok(true);
            }
            tokio::select! {
                biased;
                _ = self.token.cancelled() => return Ok(false),
                _ = settings.changed() => {}
            }
        }
    }
    pub(crate) async fn wait_until_resumed(&self) -> bool {
        let mut settings = self.settings.subscribe();
        loop {
            if self.token.is_cancelled() {
                return false;
            }
            if !settings.borrow_and_update().paused {
                return true;
            }
            tokio::select! {biased; _=self.token.cancelled()=>return false, _=settings.changed()=>{}}
        }
    }
    pub(crate) async fn pause_requested(&self) {
        let mut settings = self.settings.subscribe();
        loop {
            if settings.borrow_and_update().paused {
                return;
            }
            let _ = settings.changed().await;
        }
    }
    /// Charge each small slice before delivery. Config changes, pause and cancel
    /// interrupt the wait; idle time never builds up a large burst allowance.
    async fn pace(&self, bytes: usize) -> bool {
        let mut settings = self.settings.subscribe();
        let start = tokio::time::Instant::now();
        loop {
            if self.token.is_cancelled() {
                return false;
            }
            let config = *settings.borrow_and_update();
            if config.paused {
                return false;
            }
            if config.bytes_per_second == 0 {
                return true;
            }
            let delay = Duration::from_secs_f64(bytes as f64 / config.bytes_per_second as f64);
            tokio::select! {
                biased;
                _ = self.token.cancelled() => return false,
                _ = settings.changed() => {},
                _ = tokio::time::sleep_until(start + delay) => return true,
            }
        }
    }
    /// Cancellation and the final no-replace operation share one linearization
    /// point. Once the file is committed, a late Stop cannot change its outcome.
    pub fn try_cancel(&self) -> bool {
        let mut state = self.commit.lock().unwrap_or_else(|p| p.into_inner());
        if *state == CommitState::Committed {
            return false;
        }
        *state = CommitState::Cancelled;
        self.token.cancel();
        true
    }
    pub(crate) fn commit_file(
        &self,
        operation: impl FnOnce() -> std::io::Result<()>,
    ) -> std::io::Result<bool> {
        let mut state = self.commit.lock().unwrap_or_else(|p| p.into_inner());
        if *state == CommitState::Cancelled || self.token.is_cancelled() {
            *state = CommitState::Cancelled;
            return Ok(false);
        }
        operation()?;
        *state = CommitState::Committed;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cancellation_before_commit_prevents_file_operation() {
        let control = TransferControl::new(CancellationToken::new());
        assert!(control.try_cancel());
        assert!(!control
            .commit_file(|| panic!("cancelled file must not be committed"))
            .unwrap());
    }
    #[test]
    fn cancellation_after_commit_cannot_override_completed_output() {
        let control = TransferControl::new(CancellationToken::new());
        assert!(control.commit_file(|| Ok(())).unwrap());
        assert!(!control.try_cancel());
        assert!(!control.token().is_cancelled());
    }
}

pub async fn transfer(
    request: DownloadRequest,
    cancel: CancellationToken,
    progress: ProgressSink,
) -> Result<TransferOutcome, JobError> {
    controlled_transfer(request, TransferControl::new(cancel), progress).await
}

pub async fn controlled_transfer(
    request: DownloadRequest,
    control: TransferControl,
    progress: ProgressSink,
) -> Result<TransferOutcome, JobError> {
    if request.media.is_some() {
        return crate::media::controlled_transfer(request, control, progress).await;
    }
    let cancel = control.token();
    if cancel.is_cancelled() {
        return Ok(TransferOutcome::Cancelled);
    }
    let mut output = OutputFile::durable_prepare(&request).await?;
    let result = match segmented::run(&request, &control, progress.clone(), &mut output).await {
        Ok(Some(outcome)) => Ok(outcome),
        Ok(None) => stream(&request, &control, progress.clone(), &mut output).await,
        Err(error) => Err(error),
    };
    output.adopt_recovery(control.recovery());
    if control.recovery().is_some() && (control.preserving() || result.is_err()) {
        output.checkpoint().await?;
        return match result {
            Err(error) => Err(output.preserved_error(error)),
            _ => Ok(TransferOutcome::Cancelled),
        };
    }
    match result {
        Ok(TransferOutcome::Completed { .. }) if cancel.is_cancelled() => {
            output.abort().await?;
            if let Some(recovery) = control.recovery() {
                recovery.clear()?;
            }
            Ok(TransferOutcome::Cancelled)
        }
        Ok(done @ TransferOutcome::Completed { bytes }) => {
            if !control
                .await_confirmation(&mut output, &progress, bytes)
                .await?
            {
                if control.preserving() {
                    output.checkpoint().await?;
                    return Ok(TransferOutcome::Cancelled);
                }
                output.abort().await?;
                if let Some(r) = control.recovery() {
                    r.clear()?;
                }
                return Ok(TransferOutcome::Cancelled);
            }
            if output
                .finalize_with_progress(control, Some(progress))
                .await?
            {
                Ok(done)
            } else {
                Ok(TransferOutcome::Cancelled)
            }
        }
        Ok(TransferOutcome::Cancelled) => {
            output.abort().await?;
            if let Some(recovery) = control.recovery() {
                recovery.clear()?;
            }
            Ok(TransferOutcome::Cancelled)
        }
        Err(error) => {
            output.abort().await.map_err(|cleanup| JobError {
                message: format!("{} {}", error.message, cleanup.message),
                ..cleanup
            })?;
            Err(error)
        }
    }
}

async fn stream(
    request: &DownloadRequest,
    control: &TransferControl,
    progress: ProgressSink,
    output: &mut OutputFile,
) -> Result<TransferOutcome, JobError> {
    // System proxy discovery is reqwest's system-proxy feature; no custom proxy,
    // referer, credentials logging, certificate bypass or application retries.
    request
        .options
        .validate()
        .map_err(|e| JobError::new(ErrorKind::InvalidRequest, e))?;
    let client = reqwest::Client::builder()
        .user_agent(request.options.user_agent.as_str())
        .redirect(reqwest::redirect::Policy::limited(10))
        .referer(false)
        .no_gzip()
        .no_brotli()
        .no_deflate()
        .no_zstd()
        .retry(reqwest::retry::never())
        .connect_timeout(Duration::from_secs(30))
        .build()
        .map_err(|_| JobError::new(ErrorKind::Network, "Cannot initialize the HTTP client."))?;
    let scoped_client = refresh::client()?;
    let cancel = control.token();
    let saved = request.recovery.as_ref().and_then(|r| r.checkpoint());
    let mut received = saved.as_ref().map_or(0, |c| c.durable);
    let mut total = saved.as_ref().and_then(|c| c.total);
    let mut validator = saved.as_ref().and_then(|c| c.etag.clone());
    let mut representation_url = saved
        .as_ref()
        .and_then(|c| c.response_url.as_deref())
        .and_then(|s| url::Url::parse(s).ok());
    let mut refresh_pending = saved.as_ref().is_some_and(|c| c.refresh_pending);
    let mut strict = request.recovery.is_some();
    if received > 0 && (total.is_none() || representation_url.is_none()) {
        return Err(JobError::new(
            ErrorKind::RestartRequired,
            "Restart required: no safe continuation validator is available.",
        ));
    }
    'connect: loop {
        if output.adopt_recovery(control.recovery()) {
            output.checkpoint().await?;
        }
        strict |= control.recovery().is_some();
        if control.settings.borrow().paused {
            output.checkpoint().await?;
            progress(TransferProgress {
                received,
                total,
                resume_capable: validator.is_some(),
                paused: true,
            });
            if !control.wait_until_resumed_output(output).await? {
                return Ok(TransferOutcome::Cancelled);
            }
        }
        if cancel.is_cancelled() {
            return Ok(TransferOutcome::Cancelled);
        }
        // A fully received known-length body needs no request after an EOF pause.
        if total == Some(received) {
            return Ok(TransferOutcome::Completed { bytes: received });
        }
        if received > 0 && total.is_none() {
            if strict {
                return Err(JobError::new(
                    ErrorKind::RestartRequired,
                    "Restart required: the saved response has no known total size.",
                ));
            }
            output.restart().await?;
            received = 0;
            validator = None;
            representation_url = None;
            output.representation(None, None, None);
        }
        if received > 0 && (refresh_pending || validator.is_none()) {
            let samples = output.refresh_samples(received).await?;
            let candidate = RefreshCandidate {
                url: request.url.as_str().to_owned(),
                source_page: request.source_page.clone(),
                request_context: request.request_context.clone(),
            };
            let verified = refresh::verify_remote_samples(
                &scoped_client,
                &candidate,
                total.expect("known total"),
                &samples,
            )
            .await?;
            representation_url = url::Url::parse(&verified.final_url).ok();
            validator = verified.etag;
            refresh_pending = false;
            output.representation(total, Some(verified.final_url), validator.clone());
            output.clear_refresh_pending();
            output.checkpoint().await?;
        }
        let resume = received > 0 && total.is_some();
        let mut builder = client
            .get(request.url.clone())
            .header(reqwest::header::ACCEPT_ENCODING, "identity");
        if let Some(authorization) = &request.authorization {
            builder = builder.basic_auth(&authorization.login, Some(&authorization.password));
        }
        if resume {
            builder = builder.header(reqwest::header::RANGE, format!("bytes={received}-"));
            if let Some(validator) = validator.as_deref() {
                builder = builder.header(reqwest::header::IF_RANGE, validator);
            }
        }
        let mut response = if !request.request_context.is_empty() {
            tokio::select! {
                biased;
                _ = cancel.cancelled() => return Ok(TransferOutcome::Cancelled),
                _ = control.pause_requested() => continue 'connect,
                result = async {
                    if resume {
                        refresh::fetch_range(&scoped_client, &request.url, &request.request_context, received, None, validator.as_deref()).await
                    } else {
                        refresh::fetch_get(&scoped_client, &request.url, &request.request_context).await
                    }
                } => result?,
            }
        } else {
            tokio::select! {
                biased;
                _ = cancel.cancelled() => return Ok(TransferOutcome::Cancelled),
                _ = control.pause_requested() => continue 'connect,
                response = tokio::time::timeout(Duration::from_secs(30), builder.send()) => response.ok().and_then(Result::ok).ok_or_else(|| JobError::new(ErrorKind::Network, "Connection failed, timed out, or exceeded the redirect limit. Check the URL, network and TLS certificate."))?,
            }
        };
        if !request.request_context.is_empty()
            && (response.status() == reqwest::StatusCode::UNAUTHORIZED
                || response.status() == reqwest::StatusCode::FORBIDDEN)
        {
            return Err(JobError::new(
                ErrorKind::Http,
                "Browser authorization expired; capture the download again.",
            ));
        }
        if !response.status().is_success()
            || (response.status() == reqwest::StatusCode::PARTIAL_CONTENT && !resume)
        {
            return Err(JobError::new(
                ErrorKind::Http,
                format!(
                    "Server returned HTTP {}. A complete response is required.",
                    response.status().as_u16()
                ),
            ));
        }
        if response
            .headers()
            .get_all(reqwest::header::CONTENT_ENCODING)
            .iter()
            .any(|value| {
                value
                    .to_str()
                    .map_or(true, |s| !s.eq_ignore_ascii_case("identity"))
            })
        {
            return Err(JobError::new(ErrorKind::Http, "The server returned an unsupported encoded representation. No decompression is applied."));
        }
        if resume && response.status() == reqwest::StatusCode::PARTIAL_CONTENT {
            if representation_url.as_ref() != Some(response.url())
                || if let Some(validator) = validator.as_deref() {
                    !valid_resume(
                        response.headers(),
                        received,
                        total.expect("known total"),
                        validator,
                        response.content_length(),
                    )
                } else {
                    !refresh::exact_range(
                        response.headers(),
                        received,
                        total.expect("known total") - 1,
                        total.expect("known total"),
                        response.content_length(),
                    )
                }
            {
                return Err(JobError::new(if strict {ErrorKind::RestartRequired}else{ErrorKind::Http}, "Restart required: the server changed the file or returned an invalid resume range."));
            }
        } else {
            // Range ignored, representation changed, or no safe validator: replace
            // the owned partial contents rather than append incompatible bytes.
            if received != 0 {
                if strict {
                    return Err(JobError::new(
                        ErrorKind::RestartRequired,
                        "The server ignored the requested resume byte range. Saved bytes were preserved.",
                    ));
                }
                output.restart().await?;
            }
            received = 0;
            total = response.content_length();
            validator = total.and_then(|_| strong_etag(response.headers()));
            representation_url = Some(response.url().clone());
        }
        if request.options.server_creation_time {
            output.server_creation_time(
                response
                    .headers()
                    .get(reqwest::header::LAST_MODIFIED)
                    .and_then(|h| h.to_str().ok()),
            );
        }
        output.representation(
            total,
            representation_url.as_ref().map(|u| u.as_str().to_owned()),
            validator.clone(),
        );
        output.checkpoint().await?;
        let mut checkpoint_at = tokio::time::Instant::now();
        progress(TransferProgress {
            received,
            total,
            resume_capable: validator.is_some(),
            paused: false,
        });
        loop {
            let read_deadline = tokio::time::Instant::now() + Duration::from_secs(30);
            let chunk = loop {
                tokio::select! {
                    biased;
                    _=cancel.cancelled()=>return Ok(TransferOutcome::Cancelled),
                    _=control.pause_requested()=>continue 'connect,
                    _=tokio::time::sleep_until(checkpoint_at+Duration::from_secs(5)), if control.recovery().is_some()=>{output.checkpoint().await?;checkpoint_at=tokio::time::Instant::now();},
                    chunk=tokio::time::timeout_at(read_deadline,response.chunk())=>break chunk.map_err(|_|JobError::new(ErrorKind::Network,"The response made no progress for 30 seconds."))?.map_err(|_|JobError::new(ErrorKind::Network,"The response was interrupted."))?,
                }
            };
            let Some(chunk) = chunk else {
                break;
            };
            let next = received.checked_add(chunk.len() as u64).ok_or_else(|| {
                JobError::new(
                    ErrorKind::LengthMismatch,
                    "The response length exceeds supported file sizes.",
                )
            })?;
            if total.is_some_and(|total| next > total) {
                return Err(JobError::new(
                    ErrorKind::LengthMismatch,
                    "The response exceeded its declared length.",
                ));
            }
            let mut offset = 0;
            while offset < chunk.len() {
                let limit = control.settings.borrow().bytes_per_second;
                let quantum = if limit == 0 {
                    16_384
                } else {
                    (limit / 10).clamp(1, 16_384) as usize
                };
                let end = (offset + quantum).min(chunk.len());
                if !control.pace(end - offset).await {
                    if cancel.is_cancelled() {
                        return Ok(TransferOutcome::Cancelled);
                    }
                    continue 'connect;
                }
                output.write(&chunk[offset..end]).await?;
                received += (end - offset) as u64;
                if checkpoint_at.elapsed() >= Duration::from_secs(5) {
                    output.checkpoint().await?;
                    checkpoint_at = tokio::time::Instant::now();
                }
                offset = end;
                progress(TransferProgress {
                    received,
                    total,
                    resume_capable: validator.is_some(),
                    paused: false,
                });
            }
        }
        if total.is_some_and(|total| received != total) {
            return Err(JobError::new(
                ErrorKind::LengthMismatch,
                "The response ended before its declared length.",
            ));
        }
        if control.settings.borrow().paused {
            continue 'connect;
        }
        return Ok(TransferOutcome::Completed { bytes: received });
    }
}

fn strong_etag(headers: &HeaderMap) -> Option<String> {
    let value = headers.get(ETAG)?.to_str().ok()?;
    let opaque = value.strip_prefix('"')?.strip_suffix('"')?;
    if !opaque
        .bytes()
        .all(|b| b == 0x21 || (0x23..=0x7e).contains(&b) || b >= 0x80)
    {
        return None;
    }
    Some(value.to_owned())
}
fn valid_resume(
    headers: &HeaderMap,
    offset: u64,
    total: u64,
    validator: &str,
    length: Option<u64>,
) -> bool {
    if strong_etag(headers).as_deref() != Some(validator) {
        return false;
    }
    let Some(value) = headers
        .get(CONTENT_RANGE)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("bytes "))
    else {
        return false;
    };
    let Some((range, size)) = value.split_once('/') else {
        return false;
    };
    let Some((start, end)) = range.split_once('-') else {
        return false;
    };
    let parse = |s: &str| {
        if !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()) {
            s.parse::<u64>().ok()
        } else {
            None
        }
    };
    total > offset
        && parse(start) == Some(offset)
        && parse(end) == total.checked_sub(1)
        && parse(size) == Some(total)
        && length.is_none_or(|n| n == total - offset)
}

#[cfg(test)]
#[path = "resume_tests.rs"]
mod resume_tests;
