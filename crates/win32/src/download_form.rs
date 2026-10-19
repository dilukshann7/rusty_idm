use crate::settings::Settings;
use rusty_engine::{validate_request, ErrorKind, JobError, ReplacementPolicy};
use std::path::{Path, PathBuf};

pub const CATEGORIES: [&str; 6] = [
    "General",
    "Compressed",
    "Documents",
    "Music",
    "Programs",
    "Video",
];

pub fn suggested_filename(address: &str) -> Result<String, JobError> {
    let parsed = url::Url::parse(address.trim()).map_err(|_| {
        JobError::new(
            ErrorKind::InvalidRequest,
            "Enter a valid HTTP or HTTPS address.",
        )
    })?;
    if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
        return Err(JobError::new(
            ErrorKind::InvalidRequest,
            "Enter a valid HTTP or HTTPS address.",
        ));
    }
    let encoded = parsed
        .path_segments()
        .and_then(|mut segments| segments.next_back())
        .filter(|s| !s.is_empty())
        .unwrap_or("download.bin");
    let decoded = percent_encoding::percent_decode_str(encoded).decode_utf8_lossy();
    let name: String = decoded
        .chars()
        .map(|c| {
            if c.is_control() || "<>:\"/\\|?*".contains(c) {
                '_'
            } else {
                c
            }
        })
        .collect();
    let name = name.trim_end_matches(['.', ' ']);
    if name.is_empty()
        || validate_request(address, PathBuf::from(name), ReplacementPolicy::Deny).is_err()
    {
        return Ok("download.bin".into());
    }
    Ok(name.to_owned())
}
pub fn folder_for<'a>(settings: &'a Settings, category: &str) -> &'a Path {
    settings
        .category_destinations
        .get(&category.trim().to_lowercase())
        .map(PathBuf::as_path)
        .unwrap_or(&settings.default_destination)
}
pub fn remember_folder(settings: &mut Settings, category: &str, destination: &Path) {
    if let Some(parent) = destination.parent() {
        settings
            .category_destinations
            .insert(category.trim().to_lowercase(), parent.to_owned());
    }
}

#[cfg(test)]
#[path = "download_form_tests.rs"]
mod tests;
