//! Shared classification. A manual override never changes a destination.
use std::path::Path;
pub const BUILTIN: [&str; 5] = ["Compressed", "Documents", "Music", "Programs", "Video"];
pub fn infer(path: &Path) -> Option<&'static str> {
    let extension = path.extension()?.to_str()?.to_ascii_lowercase();
    Some(match extension.as_str() {
        "zip" | "7z" | "rar" | "tar" | "gz" | "bz2" | "xz" | "tgz" | "zst" | "cab" | "iso" => {
            "Compressed"
        }
        "pdf" | "txt" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "odt" | "ods"
        | "odp" | "rtf" | "csv" | "epub" | "md" | "html" | "htm" | "xhtml" => "Documents",
        "mp3" | "wav" | "flac" | "m4a" | "aac" | "ogg" | "opus" | "wma" | "aiff" => "Music",
        "exe" | "msi" | "msix" | "msixbundle" | "appx" | "appxbundle" | "com" | "bat" | "cmd" => {
            "Programs"
        }
        "mp4" | "mkv" | "webm" | "avi" | "mov" | "wmv" | "m4v" | "mpeg" | "mpg" | "ts" | "flv"
        | "3gp" => "Video",
        _ => return None,
    })
}
pub fn effective<'a>(path: &Path, manual: Option<&'a str>) -> Option<&'a str> {
    manual
        .filter(|s| !s.eq_ignore_ascii_case("Automatic"))
        .or_else(|| infer(path))
}
