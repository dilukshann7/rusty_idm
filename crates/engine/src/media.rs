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

const MAX_MEDIA_VARIANTS: usize = 128;

fn bitrate(f: &Value, field: &str) -> f64 {
    f[field]
        .as_f64()
        .filter(|rate| rate.is_finite() && *rate > 0.0)
        .or_else(|| {
            f["tbr"]
                .as_f64()
                .filter(|rate| rate.is_finite() && *rate > 0.0)
        })
        .unwrap_or(0.0)
}

fn compatible_audio(video_ext: &str, audio: &Value) -> bool {
    let extension = audio["ext"].as_str().unwrap_or("");
    let codec = audio["acodec"].as_str().unwrap_or("");
    match video_ext {
        "mp4" => {
            matches!(extension, "m4a" | "mp4") && (codec == "aac" || codec.starts_with("mp4a"))
        }
        "webm" => extension == "webm" && matches!(codec, "opus" | "vorbis"),
        _ => false,
    }
}

fn frame_rate(value: f64) -> Option<u32> {
    (value.is_finite() && value > 0.0 && value <= 1000.0).then(|| value.round().max(1.0) as u32)
}

fn quality_key(f: &Value) -> (u32, u32, String) {
    // YouTube's source label describes its quality tier even for cropped video.
    // Only accept a complete numeric p-label, not arbitrary format-note text.
    let note = f["format_note"].as_str().unwrap_or("");
    let source_label = note
        .split(|c: char| !c.is_ascii_alphanumeric())
        .find_map(|token| {
            let (resolution, fps) = token.split_once('p')?;
            let resolution = resolution
                .parse::<u32>()
                .ok()
                .filter(|n| (1..=16384).contains(n))?;
            let fps = if fps.is_empty() {
                None
            } else {
                Some(fps.parse::<u32>().ok().filter(|n| (1..=1000).contains(n))?)
            };
            Some((resolution, fps))
        });
    let height = f["height"].as_u64().filter(|n| (1..=16384).contains(n));
    let width = f["width"].as_u64().filter(|n| (1..=16384).contains(n));
    let resolution = source_label
        .map(|(resolution, _)| resolution)
        .unwrap_or_else(|| {
            height
                .map(|height| height.min(width.unwrap_or(height)) as u32)
                .unwrap_or(0)
        });
    let fps = f["fps"]
        .as_f64()
        .and_then(frame_rate)
        .or_else(|| source_label.and_then(|(_, fps)| fps))
        .unwrap_or(0);
    let quality = if resolution == 0 {
        let note = clean(note, 32);
        if note.is_empty() {
            "video".into()
        } else {
            note
        }
    } else if fps == 0 {
        format!("{resolution}p")
    } else {
        format!("{resolution}p{fps}")
    };
    (resolution, fps, quality)
}

fn container_rank(container: &str) -> u8 {
    match container {
        "mp4" => 0,
        "webm" => 1,
        _ => 2,
    }
}

struct PreviewCandidate {
    variant: MediaVariant,
    h264: bool,
    bitrate: f64,
}
impl PreviewCandidate {
    fn preference(&self, other: &Self) -> std::cmp::Ordering {
        container_rank(&self.variant.container)
            .cmp(&container_rank(&other.variant.container))
            .then_with(|| other.h264.cmp(&self.h264))
            .then_with(|| other.bitrate.total_cmp(&self.bitrate))
            .then_with(|| self.variant.id.cmp(&other.variant.id))
    }
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
    let audio_tracks: Vec<_> = formats
        .iter()
        .filter(|f| {
            !has_video(f)
                && has_audio(f)
                && f["has_drm"].as_bool() != Some(true)
                && id(&f["format_id"]).is_some()
        })
        .collect();
    let preferred_audio = |extension: &str| {
        audio_tracks.iter().copied().max_by(|a, b| {
            compatible_audio(extension, a)
                .cmp(&compatible_audio(extension, b))
                .then_with(|| bitrate(a, "abr").total_cmp(&bitrate(b, "abr")))
                .then_with(|| b["format_id"].as_str().cmp(&a["format_id"].as_str()))
        })
    };
    let mp4_audio = preferred_audio("mp4");
    let webm_audio = preferred_audio("webm");
    let fallback_audio = preferred_audio("");
    let mut candidates = std::collections::BTreeMap::new();
    for f in formats
        .iter()
        .filter(|f| has_video(f) && f["has_drm"].as_bool() != Some(true))
    {
        let Some(video_id) = id(&f["format_id"]) else {
            continue;
        };
        let extension = f["ext"].as_str().unwrap_or("mkv");
        let best_audio = match extension {
            "mp4" => mp4_audio,
            "webm" => webm_audio,
            _ => fallback_audio,
        };
        let (selector, container, audio, bytes) = if has_audio(f) {
            (
                video_id,
                extension.to_owned(),
                clean(f["acodec"].as_str().unwrap_or("unknown"), 32),
                size(f),
            )
        } else if let Some(track) = best_audio {
            let audio_id = id(&track["format_id"]).expect("filtered");
            let container = if compatible_audio(extension, track) {
                extension
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
        let (resolution, fps, quality) = quality_key(f);
        let codec = f["vcodec"].as_str().unwrap_or("");
        let candidate = PreviewCandidate {
            variant: MediaVariant {
                id: selector,
                quality: quality.clone(),
                container,
                audio,
                size: bytes,
            },
            h264: codec == "h264" || codec.starts_with("avc1") || codec.starts_with("avc3"),
            bitrate: bitrate(f, "vbr"),
        };
        let key = (resolution, fps, quality);
        match candidates.entry(key) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(candidate);
            }
            std::collections::btree_map::Entry::Occupied(mut entry) => {
                if candidate.preference(entry.get()).is_lt() {
                    entry.insert(candidate);
                }
            }
        }
    }
    if candidates.len() > MAX_MEDIA_VARIANTS {
        return Err(media_error("Too many video quality choices are available."));
    }
    let mut candidates: Vec<_> = candidates.into_iter().collect();
    candidates.sort_by_key(|((resolution, fps, quality), candidate)| {
        (
            candidate.variant.container != "mp4",
            std::cmp::Reverse(*resolution),
            std::cmp::Reverse(*fps),
            quality.clone(),
        )
    });
    let variants: Vec<_> = candidates
        .into_iter()
        .map(|(_, candidate)| candidate.variant)
        .collect();
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

    fn preview_formats(formats: Value) -> Result<MediaPreview, JobError> {
        parse_preview(
            "https://www.youtube.com/watch?v=abcdefghijk",
            MediaKind::Youtube,
            &json!({"id":"abcdefghijk", "title":"Sample", "formats":formats}),
        )
    }

    #[test]
    fn mp4_uses_aac_even_when_opus_has_a_higher_bitrate() {
        let preview = preview_formats(json!([
            {"format_id":"aac-low", "ext":"m4a", "vcodec":"none", "acodec":"mp4a.40.2", "abr":128.1, "filesize":100},
            {"format_id":"aac-best", "ext":"m4a", "vcodec":"none", "acodec":"mp4a.40.2", "abr":128.9, "filesize":110},
            {"format_id":"opus", "ext":"webm", "vcodec":"none", "acodec":"opus", "abr":160.5, "filesize":200},
            {"format_id":"video", "ext":"mp4", "vcodec":"avc1", "acodec":"none", "height":720, "filesize":900}
        ])).unwrap();
        assert_eq!(preview.variants.len(), 1);
        assert_eq!(preview.variants[0].id, "video+aac-best");
        assert_eq!(preview.variants[0].container, "mp4");
        assert_eq!(preview.variants[0].audio, "mp4a.40.2");
        assert_eq!(preview.variants[0].size, Some(1010));
    }

    #[test]
    fn webm_pairs_compatible_audio_and_mp4_can_fall_back_to_mkv() {
        let preview = preview_formats(json!([
            {"format_id":"aac", "ext":"m4a", "vcodec":"none", "acodec":"mp4a.40.2", "abr":256},
            {"format_id":"opus", "ext":"webm", "vcodec":"none", "acodec":"opus", "abr":128},
            {"format_id":"webm", "ext":"webm", "vcodec":"vp9", "acodec":"none", "height":1080}
        ]))
        .unwrap();
        assert_eq!(preview.variants[0].id, "webm+opus");
        assert_eq!(preview.variants[0].container, "webm");
        let fallback = preview_formats(json!([
            {"format_id":"opus", "ext":"webm", "vcodec":"none", "acodec":"opus", "abr":128},
            {"format_id":"video", "ext":"mp4", "vcodec":"avc1", "acodec":"none", "height":720}
        ]))
        .unwrap();
        assert_eq!(fallback.variants[0].id, "video+opus");
        assert_eq!(fallback.variants[0].container, "mkv");
    }

    #[test]
    fn mp4_choices_come_first_without_losing_higher_webm_resolutions_or_frame_rates() {
        let preview = preview_formats(json!([
            {"format_id":"webm-1080", "ext":"webm", "vcodec":"vp9", "acodec":"opus", "height":1080, "fps":60},
            {"format_id":"webm-720", "ext":"webm", "vcodec":"vp9", "acodec":"opus", "height":720, "fps":60},
            {"format_id":"mp4-360", "ext":"mp4", "vcodec":"avc1", "acodec":"aac", "height":360},
            {"format_id":"mp4-720-30", "ext":"mp4", "vcodec":"avc1", "acodec":"aac", "height":720, "fps":29.97},
            {"format_id":"mp4-720-60", "ext":"mp4", "vcodec":"avc1", "acodec":"aac", "height":720, "fps":59.94}
        ])).unwrap();
        let choices: Vec<_> = preview
            .variants
            .iter()
            .map(|v| (v.id.as_str(), v.quality.as_str()))
            .collect();
        assert_eq!(
            choices,
            vec![
                ("mp4-720-60", "720p60"),
                ("mp4-720-30", "720p30"),
                ("mp4-360", "360p"),
                ("webm-1080", "1080p60")
            ]
        );
    }

    #[test]
    fn duplicate_choices_prefer_container_then_h264_then_bitrate_then_format_id() {
        let formats = json!([
            {"format_id":"webm", "ext":"webm", "vcodec":"vp9", "acodec":"opus", "height":720, "tbr":9000},
            {"format_id":"av1", "ext":"mp4", "vcodec":"av01", "acodec":"aac", "height":720, "tbr":8000},
            {"format_id":"h264-low", "ext":"mp4", "vcodec":"avc1", "acodec":"aac", "height":720, "tbr":1000.1},
            {"format_id":"h264-b", "ext":"mp4", "vcodec":"h264", "acodec":"aac", "height":720, "tbr":1000.9},
            {"format_id":"h264-a", "ext":"mp4", "vcodec":"avc1", "acodec":"aac", "height":720, "tbr":1000.9},
            {"format_id":"mkv", "ext":"mkv", "vcodec":"h264", "acodec":"aac", "height":720, "tbr":10000}
        ]);
        let preview = preview_formats(formats.clone()).unwrap();
        assert_eq!(preview.variants.len(), 1);
        assert_eq!(preview.variants[0].id, "h264-a");
        let mut reversed = formats.as_array().unwrap().clone();
        reversed.reverse();
        assert_eq!(
            preview_formats(json!(reversed)).unwrap().variants,
            preview.variants
        );
    }

    #[test]
    fn source_labels_override_cropped_heights_and_dimensions_handle_portrait_video() {
        let preview = preview_formats(json!([
            {"format_id":"cropped", "ext":"mp4", "vcodec":"avc1", "acodec":"aac", "width":3840, "height":1920, "format_note":"2160p60 HDR", "fps":59.94},
            {"format_id":"portrait", "ext":"mp4", "vcodec":"avc1", "acodec":"aac", "width":720, "height":1280, "fps":29.97},
            {"format_id":"label-only", "ext":"mp4", "vcodec":"avc1", "acodec":"aac", "format_note":"1440p50, premium"},
            {"format_id":"height-only", "ext":"mp4", "vcodec":"avc1", "acodec":"aac", "height":360},
            {"format_id":"unknown", "ext":"mp4", "format_note":"public stream"}
        ])).unwrap();
        let labels: Vec<_> = preview
            .variants
            .iter()
            .map(|v| v.quality.as_str())
            .collect();
        assert_eq!(
            labels,
            vec!["2160p60", "1440p50", "720p30", "360p", "public stream"]
        );
    }

    #[test]
    fn unavailable_or_invalid_quality_metadata_does_not_invent_a_resolution() {
        let preview = preview_formats(json!([
            {"format_id":"unknown", "ext":"mp4", "width":1920, "height":0, "fps":-1},
            {"format_id":"malformed-note", "ext":"mp4", "height":480, "format_note":"1080preview", "fps":0}
        ])).unwrap();
        assert_eq!(preview.variants[0].quality, "480p");
        assert_eq!(preview.variants[1].quality, "video");
    }

    #[test]
    fn audio_ties_are_deterministic_and_drm_tracks_are_excluded() {
        let formats = json!([
            {"format_id":"aac-b", "ext":"m4a", "vcodec":"none", "acodec":"aac", "abr":128},
            {"format_id":"aac-a", "ext":"m4a", "vcodec":"none", "acodec":"aac", "abr":128},
            {"format_id":"drm", "ext":"m4a", "vcodec":"none", "acodec":"aac", "abr":300, "has_drm":true},
            {"format_id":"video", "ext":"mp4", "vcodec":"avc1", "acodec":"none", "height":720}
        ]);
        let preview = preview_formats(formats.clone()).unwrap();
        assert_eq!(preview.variants[0].id, "video+aac-a");
        let mut reversed = formats.as_array().unwrap().clone();
        reversed.reverse();
        assert_eq!(
            preview_formats(json!(reversed)).unwrap().variants,
            preview.variants
        );
    }

    #[test]
    fn preview_limit_applies_after_deduplication_and_never_silently_drops_qualities() {
        let mut formats: Vec<Value> = (0..128)
            .map(|index| {
                json!({
                    "format_id":format!("video-{index}"), "ext":"mp4", "height":144 + index
                })
            })
            .collect();
        let preview = preview_formats(json!(formats)).unwrap();
        assert_eq!(preview.variants.len(), 128);
        assert_eq!(preview.variants.last().unwrap().quality, "144p");
        formats.push(json!({"format_id":"duplicate", "ext":"mp4", "height":144}));
        assert_eq!(preview_formats(json!(formats)).unwrap().variants.len(), 128);
        formats.push(json!({"format_id":"overflow", "ext":"mp4", "height":1000}));
        assert!(preview_formats(json!(formats)).is_err());
    }

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
                            details: None,
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
                details: None,
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
                details: None,
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
            details: None,
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
