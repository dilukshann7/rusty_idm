//! Public, single-video inspection and transfer. Tool output is untrusted and bounded.
use crate::{
    output::OutputFile,
    transfer::{ProgressSink, TransferControl, TransferOutcome, TransferProgress},
    DownloadRequest, ErrorKind, JobError,
};
use rusty_browser_bridge::protocol::{MediaKind, MediaVariant};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    io::Read,
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio_util::sync::CancellationToken;

const YT_DLP_HASH: &str = "66674953fe251b89f4d08c5f0e35e0728679bd67ab3d7d05c0562af101dd3e7a";
const DENO_HASH: &str = "e020f3e232bd16e33768dee528e5983349c962952051ced0a5d58ad42f5d9b33";
const FFMPEG_HASH: &str = "63fc73812f8dddbe5ecc3e0cbe50c597ce7c9845fe8562bc4ff9d14a9d9c48ea";
const FFPROBE_HASH: &str = "21ac55ed7fccb092980d6583a784058ae1e42ad713d422de8e771f26e46a5611";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MediaSelection {
    pub identity: String,
    pub format_id: String,
    pub container: String,
    pub quality: String,
    pub audio: String,
    pub kind: MediaKind,
}
impl MediaSelection {
    pub fn validate(&self) -> Result<(), JobError> {
        if self.identity.is_empty()
            || self.identity.len() > 160
            || self.identity.chars().any(char::is_control)
            || self.format_id.is_empty()
            || self.format_id.len() > 96
            || !self
                .format_id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '+'))
            || !["mp4", "webm", "mkv"].contains(&self.container.as_str())
            || self.quality.is_empty()
            || self.quality.len() > 32
            || self.quality.chars().any(char::is_control)
            || self.audio.is_empty()
            || self.audio.len() > 32
            || self.audio.chars().any(char::is_control)
        {
            return Err(media_error("Invalid media format selection."));
        }
        Ok(())
    }
    pub fn matches(&self, preview: &MediaPreview) -> bool {
        self.identity == preview.identity
            && self.kind == preview.kind
            && preview.variants.iter().any(|variant| {
                variant.id == self.format_id
                    && variant.container == self.container
                    && variant.quality == self.quality
                    && variant.audio == self.audio
            })
    }
}

#[derive(Clone)]
pub struct MediaPreview {
    pub url: String,
    pub kind: MediaKind,
    pub identity: String,
    pub title: String,
    pub variants: Vec<MediaVariant>,
}
impl std::fmt::Debug for MediaPreview {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MediaPreview")
            .field("kind", &self.kind)
            .field("variant_count", &self.variants.len())
            .finish_non_exhaustive()
    }
}

#[derive(Clone)]
pub struct MediaTools {
    yt_dlp: PathBuf,
    ffmpeg: PathBuf,
    ffprobe: PathBuf,
    deno: PathBuf,
}

impl MediaTools {
    pub fn bundled() -> Result<Self, JobError> {
        let executable = std::env::current_exe().map_err(|_| tool_error())?;
        let root = executable
            .parent()
            .and_then(Path::parent)
            .ok_or_else(tool_error)?;
        let tools = root.join("tools");
        let result = Self {
            yt_dlp: tools.join("yt-dlp.exe"),
            ffmpeg: tools.join("ffmpeg.exe"),
            ffprobe: tools.join("ffprobe.exe"),
            deno: tools.join("deno.exe"),
        };
        for (path, prefix) in [
            (&result.yt_dlp, YT_DLP_HASH),
            (&result.deno, DENO_HASH),
            (&result.ffmpeg, FFMPEG_HASH),
            (&result.ffprobe, FFPROBE_HASH),
        ] {
            if !path.is_absolute()
                || !path.is_file()
                || !file_hash(path).is_some_and(|hash| hash == prefix)
            {
                return Err(tool_error());
            }
        }
        Ok(result)
    }
    pub fn paths(&self) -> [&Path; 4] {
        [&self.yt_dlp, &self.ffmpeg, &self.ffprobe, &self.deno]
    }
    fn args(&self) -> Vec<String> {
        vec![
            "--ignore-config".into(),
            "--no-plugin-dirs".into(),
            "--no-remote-components".into(),
            "--no-js-runtimes".into(),
            "--js-runtimes".into(),
            format!("deno:{}", self.deno.display()),
            "--no-playlist".into(),
            "--color".into(),
            "never".into(),
            "--socket-timeout".into(),
            "30".into(),
            "--ffmpeg-location".into(),
            self.ffmpeg
                .parent()
                .expect("bundled path")
                .display()
                .to_string(),
        ]
    }
}

fn tool_error() -> JobError {
    JobError::new(
        ErrorKind::InvalidRequest,
        "Bundled media tools are missing or failed verification. Reinstall Rusty.",
    )
}
fn file_hash(path: &Path) -> Option<String> {
    let mut file = std::fs::File::open(path).ok()?;
    let mut context = ring::digest::Context::new(&ring::digest::SHA256);
    let mut bytes = [0u8; 65536];
    loop {
        let n = file.read(&mut bytes).ok()?;
        if n == 0 {
            break;
        }
        context.update(&bytes[..n]);
    }
    Some(
        context
            .finish()
            .as_ref()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
    )
}
fn media_error(message: &'static str) -> JobError {
    JobError::new(ErrorKind::InvalidRequest, message)
}

async fn capped(mut reader: impl AsyncRead + Unpin, max: usize) -> Result<Vec<u8>, JobError> {
    let mut result = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        let n = reader
            .read(&mut chunk)
            .await
            .map_err(|_| media_error("Cannot read media tool output."))?;
        if n == 0 {
            return Ok(result);
        }
        if result.len() + n > max {
            return Err(media_error("Media tool output exceeded its limit."));
        }
        result.extend_from_slice(&chunk[..n]);
    }
}

async fn metadata(
    url: &str,
    tools: &MediaTools,
    cancel: CancellationToken,
) -> Result<Value, JobError> {
    let mut args = tools.args();
    args.extend([
        "--dump-single-json".into(),
        "--skip-download".into(),
        "--no-warnings".into(),
        "--".into(),
        url.into(),
    ]);
    let mut process = crate::media_process::ProcessTree::spawn(&tools.yt_dlp, &args)?;
    let stdout = process.child.stdout.take().ok_or_else(tool_error)?;
    let stderr = process.child.stderr.take().ok_or_else(tool_error)?;
    let work = async {
        let (out, err, status) = tokio::join!(
            capped(stdout, 8 * 1024 * 1024),
            capped(stderr, 128 * 1024),
            process.child.wait()
        );
        let output = out?;
        let _ = err?; // Never expose tool text: it may contain signed stream URLs.
        if !status.map_err(|_| tool_error())?.success() {
            return Err(media_error("This public video could not be inspected. It may be unavailable or require an account."));
        }
        serde_json::from_slice::<Value>(&output)
            .map_err(|_| media_error("The media tool returned invalid metadata."))
    };
    tokio::select! {
        _ = cancel.cancelled() => Err(media_error("Media inspection was cancelled.")),
        result = tokio::time::timeout(Duration::from_secs(150), work) => result.map_err(|_| media_error("Media inspection timed out."))?,
    }
}

fn clean(value: &str, limit: usize) -> String {
    let mut result = String::new();
    for character in value.chars().filter(|c| !c.is_control()) {
        if result.len() + character.len_utf8() > limit {
            break;
        }
        result.push(character);
    }
    result
}
fn id(value: &Value) -> Option<String> {
    let id = value.as_str()?;
    (id.len() <= 80
        && !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '+')))
    .then(|| id.to_owned())
}
fn has_video(f: &Value) -> bool {
    f["vcodec"].as_str() != Some("none")
}
fn has_audio(f: &Value) -> bool {
    f["acodec"].as_str() != Some("none")
}
fn size(f: &Value) -> Option<u64> {
    f["filesize"]
        .as_u64()
        .or_else(|| f["filesize_approx"].as_u64())
}

pub fn parse_preview(url: &str, kind: MediaKind, value: &Value) -> Result<MediaPreview, JobError> {
    if !value.is_object()
        || value["entries"].is_array()
        || value["is_live"].as_bool() == Some(true)
        || matches!(
            value["live_status"].as_str(),
            Some("is_live" | "is_upcoming" | "post_live")
        )
        || matches!(
            value["availability"].as_str(),
            Some("private" | "premium_only" | "subscriber_only" | "needs_auth")
        )
    {
        return Err(media_error(
            "Only single public on-demand videos are supported.",
        ));
    }
    let media_id =
        id(&value["id"]).ok_or_else(|| media_error("The media identity is unavailable."))?;
    let extractor = clean(value["extractor_key"].as_str().unwrap_or("generic"), 60);
    let identity = format!("{extractor}:{media_id}");
    let title = clean(value["title"].as_str().unwrap_or("Video"), 160);
    let formats = value["formats"]
        .as_array()
        .ok_or_else(|| media_error("No video formats are available."))?;
    let best_audio = formats
        .iter()
        .filter(|f| {
            !has_video(f)
                && has_audio(f)
                && f["has_drm"].as_bool() != Some(true)
                && id(&f["format_id"]).is_some()
        })
        .max_by_key(|f| f["abr"].as_u64().or_else(|| f["tbr"].as_u64()).unwrap_or(0));
    let mut variants = Vec::new();
    for f in formats
        .iter()
        .filter(|f| has_video(f) && f["has_drm"].as_bool() != Some(true))
    {
        let Some(video_id) = id(&f["format_id"]) else {
            continue;
        };
        let extension = f["ext"].as_str().unwrap_or("mkv");
        let (selector, container, audio, bytes) = if has_audio(f) {
            (
                video_id,
                extension.to_owned(),
                clean(f["acodec"].as_str().unwrap_or("unknown"), 32),
                size(f),
            )
        } else if let Some(track) = best_audio {
            let audio_id = id(&track["format_id"]).expect("filtered");
            let audio_ext = track["ext"].as_str().unwrap_or("");
            let container = if extension == "mp4" && audio_ext == "m4a" {
                "mp4"
            } else if extension == "webm" && audio_ext == "webm" {
                "webm"
            } else {
                "mkv"
            };
            (
                format!("{video_id}+{audio_id}"),
                container.into(),
                clean(track["acodec"].as_str().unwrap_or("audio"), 32),
                size(f).zip(size(track)).map(|(a, b)| a.saturating_add(b)),
            )
        } else {
            continue;
        };
        if selector.len() > 96 || !["mp4", "webm", "mkv"].contains(&container.as_str()) {
            continue;
        }
        let height = f["height"].as_u64().unwrap_or(0).min(16384);
        let quality = if height > 0 {
            format!("{height}p")
        } else {
            clean(f["format_note"].as_str().unwrap_or("video"), 32)
        };
        if !variants.iter().any(|v: &MediaVariant| v.id == selector) {
            variants.push(MediaVariant {
                id: selector,
                quality,
                container,
                audio,
                size: bytes,
            });
        }
    }
    variants.sort_by_key(|v| {
        std::cmp::Reverse(v.quality.trim_end_matches('p').parse::<u32>().unwrap_or(0))
    });
    variants.truncate(24);
    if variants.is_empty() {
        return Err(media_error(
            "No supported video and audio format pair is available.",
        ));
    }
    Ok(MediaPreview {
        url: url.into(),
        kind,
        identity,
        title,
        variants,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn preview_pairs_video_and_audio_and_rejects_non_public_media() {
        let metadata = json!({
            "id": "abcdefghijk", "extractor_key": "Youtube", "title": "Sample",
            "live_status": "not_live", "formats": [
                {"format_id": "140", "ext": "m4a", "vcodec": "none", "acodec": "mp4a.40.2", "filesize": 100},
                {"format_id": "137", "ext": "mp4", "vcodec": "avc1", "acodec": "none", "height": 1080, "filesize": 900},
                {"format_id": "18", "ext": "mp4", "vcodec": "avc1", "acodec": "mp4a.40.2", "height": 360}
            ]
        });
        let preview = parse_preview(
            "https://www.youtube.com/watch?v=abcdefghijk",
            MediaKind::Youtube,
            &metadata,
        )
        .unwrap();
        assert_eq!(preview.identity, "Youtube:abcdefghijk");
        assert_eq!(preview.variants[0].id, "137+140");
        assert_eq!(preview.variants[0].size, Some(1000));
        assert_eq!(preview.variants[0].container, "mp4");
        let mut live = metadata.clone();
        live["is_live"] = json!(true);
        assert!(parse_preview("https://example.com/live.m3u8", MediaKind::Hls, &live).is_err());
        let mut playlist = metadata;
        playlist["entries"] = json!([]);
        assert!(parse_preview("https://example.com/list", MediaKind::Dash, &playlist).is_err());
    }

    #[test]
    fn unknown_manifest_codecs_remain_selectable_and_formats_are_rechecked() {
        let title = clean(&"🎬".repeat(160), 160);
        assert!(title.len() <= 160);
        assert!(title.encode_utf16().count() <= 160);
        let data = json!({"id":"public", "title":"HLS", "formats":[
            {"format_id":"720", "ext":"mp4", "height":720}
        ]});
        let preview =
            parse_preview("https://example.com/master.m3u8", MediaKind::Hls, &data).unwrap();
        assert_eq!(preview.variants.len(), 1);
        assert_eq!(preview.variants[0].audio, "unknown");
        let selection = MediaSelection {
            identity: preview.identity.clone(),
            format_id: "720".into(),
            container: "mp4".into(),
            quality: "720p".into(),
            audio: "unknown".into(),
            kind: MediaKind::Hls,
        };
        assert!(selection.matches(&preview));
        let mut changed = preview.clone();
        changed.variants[0].quality = "1080p".into();
        assert!(!selection.matches(&changed));
        changed = preview.clone();
        changed.variants[0].audio = "aac".into();
        assert!(!selection.matches(&changed));
        changed = preview;
        changed.identity = "other".into();
        assert!(!selection.matches(&changed));
    }

    #[test]
    fn completed_media_requires_both_streams_before_publication() {
        assert!(verify_streams(
            &json!({"streams":[{"codec_type":"video"},{"codec_type":"audio"}]})
        )
        .is_ok());
        assert!(verify_streams(&json!({"streams":[{"codec_type":"audio"}]})).is_err());
        assert!(verify_streams(&json!({"streams":[]})).is_err());
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join("media.mp4"), []).unwrap();
        assert!(completed_output(directory.path(), "mp4").is_err());
        std::fs::write(directory.path().join("media.mp4"), b"candidate").unwrap();
        assert!(completed_output(directory.path(), "mp4").is_ok());
        assert!(completed_output(directory.path(), "webm").is_err());
    }
}

pub async fn inspect(
    url: &str,
    kind: MediaKind,
    cancel: CancellationToken,
) -> Result<MediaPreview, JobError> {
    let parsed = url::Url::parse(url).map_err(|_| media_error("Invalid media address."))?;
    if !matches!(parsed.scheme(), "http" | "https")
        || parsed.host_str().is_none()
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || url.len() > 8192
    {
        return Err(media_error("Invalid media address."));
    }
    if kind == MediaKind::Youtube {
        let host = parsed.host_str().unwrap_or_default();
        let video_id = if parsed.path() == "/watch" {
            parsed
                .query_pairs()
                .find(|(name, _)| name == "v")
                .map(|(_, value)| value.into_owned())
        } else {
            parsed
                .path()
                .strip_prefix("/shorts/")
                .map(|value| value.trim_end_matches('/').to_owned())
        };
        if !["youtube.com", "www.youtube.com", "m.youtube.com"].contains(&host)
            || !video_id.as_deref().is_some_and(|id| {
                id.len() == 11
                    && id
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
            })
        {
            return Err(media_error(
                "Only public YouTube video pages are supported.",
            ));
        }
    }
    let tools = MediaTools::bundled()?;
    let value = metadata(url, &tools, cancel).await?;
    parse_preview(url, kind, &value)
}

pub(crate) fn stage_directory(request: &DownloadRequest) -> Result<PathBuf, JobError> {
    let entry = request
        .recovery
        .as_ref()
        .ok_or_else(|| media_error("Media job has no recovery record."))?
        .entry;
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .ok_or_else(|| media_error("Local AppData is unavailable."))?;
    Ok(base
        .join("RustyIDM")
        .join("media-staging")
        .join(entry.0.to_string()))
}

fn ensure_stage(stage: &Path) -> Result<(), JobError> {
    std::fs::create_dir_all(stage)
        .map_err(|_| JobError::new(ErrorKind::Io, "Cannot create media staging directory."))?;
    let metadata = std::fs::symlink_metadata(stage)
        .map_err(|_| JobError::new(ErrorKind::Io, "Cannot inspect media staging directory."))?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(JobError::new(
            ErrorKind::DestinationConflict,
            "Media staging directory is unsafe.",
        ));
    }
    Ok(())
}

async fn progress_lines(
    mut source: impl AsyncRead + Unpin,
    sink: ProgressSink,
) -> Result<(), JobError> {
    let mut chunk = [0u8; 8192];
    let mut line = Vec::new();
    let mut seen = 0usize;
    loop {
        let count = source
            .read(&mut chunk)
            .await
            .map_err(|_| media_error("Cannot read media progress."))?;
        if count == 0 {
            return Ok(());
        }
        seen += count;
        if seen > 16 * 1024 * 1024 {
            return Err(media_error("Media progress output exceeded its limit."));
        }
        for byte in &chunk[..count] {
            if *byte == b'\n' || *byte == b'\r' {
                if let Some(record) = line.strip_prefix(b"RUSTY_PROGRESS:") {
                    if let Ok(value) = serde_json::from_slice::<Value>(record) {
                        let received = value["downloaded_bytes"].as_u64().unwrap_or(0);
                        let total = value["total_bytes"]
                            .as_u64()
                            .or_else(|| value["total_bytes_estimate"].as_u64());
                        sink(TransferProgress {
                            received,
                            total,
                            resume_capable: true,
                            paused: false,
                        });
                    }
                }
                line.clear();
            } else if line.len() < 65536 {
                line.push(*byte);
            }
        }
    }
}

async fn run_tool(
    request: &DownloadRequest,
    tools: &MediaTools,
    stage: &Path,
    control: &TransferControl,
    progress: &ProgressSink,
) -> Result<TransferOutcome, JobError> {
    let selection = request.media.as_ref().expect("media request");
    let mut args = tools.args();
    args.extend([
        "--newline".into(),
        "--progress".into(),
        "--progress-delta".into(),
        "0.2".into(),
        "--progress-template".into(),
        "download:RUSTY_PROGRESS:%(progress)j".into(),
        "--no-overwrites".into(),
        "--no-post-overwrites".into(),
        "--continue".into(),
        "--part".into(),
        "--windows-filenames".into(),
        "--retries".into(),
        "3".into(),
        "--fragment-retries".into(),
        "3".into(),
        "--format".into(),
        selection.format_id.clone(),
        "--merge-output-format".into(),
        selection.container.clone(),
        "--paths".into(),
        format!("home:{}", stage.display()),
        "--paths".into(),
        format!("temp:{}", stage.display()),
        "--output".into(),
        "media.%(ext)s".into(),
        "--".into(),
        request.url.as_str().into(),
    ]);
    let cancellation = control.token();
    loop {
        if cancellation.is_cancelled() {
            return Ok(TransferOutcome::Cancelled);
        }
        if control.is_paused() {
            progress(TransferProgress {
                received: 0,
                total: None,
                resume_capable: true,
                paused: true,
            });
            if !control.wait_until_resumed().await {
                return Ok(TransferOutcome::Cancelled);
            }
        }
        let mut process = crate::media_process::ProcessTree::spawn(&tools.yt_dlp, &args)?;
        let stdout = process.child.stdout.take().ok_or_else(tool_error)?;
        let stderr = process.child.stderr.take().ok_or_else(tool_error)?;
        let output = tokio::spawn(progress_lines(stdout, progress.clone()));
        let errors = tokio::spawn(capped(stderr, 128 * 1024));
        enum Exit {
            Cancelled,
            Paused,
            Status(std::io::Result<std::process::ExitStatus>),
        }
        let exit = tokio::select! {
            biased;
            _ = cancellation.cancelled() => Exit::Cancelled,
            _ = control.pause_requested() => Exit::Paused,
            status = process.child.wait() => Exit::Status(status),
        };
        let status = match exit {
            Exit::Cancelled => {
                process.stop().await?;
                output.abort();
                errors.abort();
                return Ok(TransferOutcome::Cancelled);
            }
            Exit::Paused => {
                process.stop().await?;
                output.abort();
                errors.abort();
                continue;
            }
            Exit::Status(status) => {
                status.map_err(|_| media_error("Media tool stopped unexpectedly."))?
            }
        };
        output
            .await
            .map_err(|_| media_error("Media progress reader stopped."))??;
        let _ = errors
            .await
            .map_err(|_| media_error("Media error reader stopped."))??;
        if !status.success() {
            return Err(media_error(
                "Media download failed. The staging files were retained.",
            ));
        }
        return Ok(TransferOutcome::Completed { bytes: 0 });
    }
}

fn completed_output(stage: &Path, container: &str) -> Result<PathBuf, JobError> {
    let path = stage.join(format!("media.{container}"));
    let info = std::fs::symlink_metadata(&path)
        .map_err(|_| media_error("The media tool did not produce the selected container."))?;
    if !info.is_file() || info.file_type().is_symlink() || info.len() == 0 {
        return Err(media_error(
            "The completed media file is missing or unsafe.",
        ));
    }
    Ok(path)
}

fn verify_streams(value: &Value) -> Result<(), JobError> {
    let streams = value["streams"]
        .as_array()
        .ok_or_else(|| media_error("Cannot verify the completed media streams."))?;
    if streams.len() > 128
        || !streams.iter().any(|stream| stream["codec_type"] == "video")
        || !streams.iter().any(|stream| stream["codec_type"] == "audio")
    {
        return Err(media_error(
            "The completed media does not contain both video and audio.",
        ));
    }
    Ok(())
}

async fn verify_output(
    tools: &MediaTools,
    path: &Path,
    cancel: CancellationToken,
) -> Result<(), JobError> {
    let args = vec![
        "-v".into(),
        "error".into(),
        "-protocol_whitelist".into(),
        "file".into(),
        "-show_entries".into(),
        "stream=codec_type".into(),
        "-of".into(),
        "json".into(),
        "-i".into(),
        path.display().to_string(),
    ];
    let mut process = crate::media_process::ProcessTree::spawn(&tools.ffprobe, &args)?;
    let stdout = process.child.stdout.take().ok_or_else(tool_error)?;
    let stderr = process.child.stderr.take().ok_or_else(tool_error)?;
    let work = async {
        let (output, errors, status) = tokio::join!(
            capped(stdout, 1024 * 1024),
            capped(stderr, 128 * 1024),
            process.child.wait()
        );
        let output = output?;
        let _ = errors?;
        if !status
            .map_err(|_| media_error("Media verification stopped unexpectedly."))?
            .success()
        {
            return Err(media_error("The completed media failed verification."));
        }
        let value = serde_json::from_slice(&output)
            .map_err(|_| media_error("Media verification returned invalid metadata."))?;
        verify_streams(&value)
    };
    tokio::select! {
        _ = cancel.cancelled() => Err(media_error("Media verification was cancelled.")),
        result = tokio::time::timeout(Duration::from_secs(30), work) => result.map_err(|_| media_error("Media verification timed out."))?,
    }
}

fn retain_stage(request: &DownloadRequest, stage: &Path) -> Result<(), JobError> {
    let recovery = request
        .recovery
        .as_ref()
        .ok_or_else(|| media_error("Media recovery record is missing."))?;
    recovery
        .store
        .update(|state| {
            let entry = state
                .entries
                .get_mut(&recovery.entry)
                .ok_or("Media recovery entry is missing.")?;
            if !entry.retained_files.iter().any(|path| path == stage) {
                entry.retained_files.push(stage.to_path_buf());
            }
            Ok(())
        })
        .map_err(crate::storage::storage_error)
}

fn clear_stage(request: &DownloadRequest, stage: &Path) -> Result<(), JobError> {
    let recovery = request
        .recovery
        .as_ref()
        .ok_or_else(|| media_error("Media recovery record is missing."))?;
    recovery
        .store
        .update(|state| {
            let entry = state
                .entries
                .get_mut(&recovery.entry)
                .ok_or("Media recovery entry is missing.")?;
            entry.retained_files.retain(|path| path != stage);
            Ok(())
        })
        .map_err(crate::storage::storage_error)
}

pub async fn controlled_transfer(
    request: DownloadRequest,
    control: TransferControl,
    progress: ProgressSink,
) -> Result<TransferOutcome, JobError> {
    let selection = request
        .media
        .as_ref()
        .ok_or_else(|| media_error("Missing media selection."))?;
    selection.validate()?;
    if request
        .destination
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
        != Some(selection.container.clone())
    {
        return Err(media_error(
            "The destination extension differs from the selected container.",
        ));
    }
    let tools = MediaTools::bundled()?;
    let current = match inspect(
        request.url.as_str(),
        selection.kind.clone(),
        control.token(),
    )
    .await
    {
        Ok(preview) => preview,
        Err(_) if control.token().is_cancelled() => return Ok(TransferOutcome::Cancelled),
        Err(error) => return Err(error),
    };
    if !selection.matches(&current) {
        return Err(media_error(
            "The video or selected format changed. Inspect it again.",
        ));
    }
    let stage = stage_directory(&request)?;
    ensure_stage(&stage)?;
    retain_stage(&request, &stage)?;
    let mut output = OutputFile::durable_prepare(&request).await?;
    match run_tool(&request, &tools, &stage, &control, &progress).await {
        Ok(TransferOutcome::Cancelled) => {
            output.adopt_recovery(control.recovery());
            output.checkpoint().await?;
            retain_stage(&request, &stage)?;
            return Ok(TransferOutcome::Cancelled);
        }
        Err(error) => {
            output.adopt_recovery(control.recovery());
            output.checkpoint().await?;
            retain_stage(&request, &stage)?;
            return Err(error.retained(stage));
        }
        Ok(TransferOutcome::Completed { .. }) => {}
    }
    let media_file = completed_output(&stage, &selection.container)
        .map_err(|error| error.retained(stage.clone()))?;
    if let Err(error) = verify_output(&tools, &media_file, control.token()).await {
        if control.token().is_cancelled() {
            return Ok(TransferOutcome::Cancelled);
        }
        return Err(error.retained(stage));
    }
    let mut file = tokio::fs::File::open(&media_file)
        .await
        .map_err(|_| media_error("Cannot open completed media staging file."))?;
    let total = file
        .metadata()
        .await
        .map_err(|_| media_error("Cannot inspect completed media file."))?
        .len();
    output.restart().await?;
    output.representation(Some(total), None, None);
    let mut copied = 0u64;
    let mut bytes = [0u8; 65536];
    loop {
        if control.token().is_cancelled() {
            output.checkpoint().await?;
            retain_stage(&request, &stage)?;
            return Ok(TransferOutcome::Cancelled);
        }
        if control.is_paused() {
            output.checkpoint().await?;
            progress(TransferProgress {
                received: copied,
                total: Some(total),
                resume_capable: true,
                paused: true,
            });
            if !control.wait_until_resumed_output(&mut output).await? {
                retain_stage(&request, &stage)?;
                return Ok(TransferOutcome::Cancelled);
            }
        }
        let count = file
            .read(&mut bytes)
            .await
            .map_err(|_| media_error("Cannot read completed media file."))?;
        if count == 0 {
            break;
        }
        output.write(&bytes[..count]).await?;
        copied += count as u64;
        progress(TransferProgress {
            received: copied,
            total: Some(total),
            resume_capable: true,
            paused: false,
        });
    }
    if copied != total {
        return Err(output.preserved_error(media_error(
            "Media staging file changed during publication.",
        )));
    }
    if !control
        .await_confirmation(&mut output, &progress, copied)
        .await?
    {
        retain_stage(&request, &stage)?;
        return Ok(TransferOutcome::Cancelled);
    }
    if output
        .finalize_with_progress(control, Some(progress))
        .await?
    {
        drop(file);
        if tokio::fs::remove_file(&media_file).await.is_ok()
            && tokio::fs::remove_dir(&stage).await.is_ok()
        {
            // Publication is already committed. Keep a stale recovery path if
            // storage cleanup fails; the next explicit removal can reconcile it.
            let _ = clear_stage(&request, &stage);
        }
        Ok(TransferOutcome::Completed { bytes: copied })
    } else {
        retain_stage(&request, &stage)?;
        Ok(TransferOutcome::Cancelled)
    }
}
