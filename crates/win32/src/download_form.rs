use crate::settings::Settings;
use rusty_engine::{validate_request, ErrorKind, JobError, ReplacementPolicy};
use std::path::{Path, PathBuf};

pub const CATEGORIES: [&str; 6] = [
    "Automatic",
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
    let name = if category.eq_ignore_ascii_case("Automatic") {
        "General"
    } else {
        category.trim()
    };
    settings
        .configuration
        .categories
        .iter()
        .find(|r| r.name.eq_ignore_ascii_case(name))
        .and_then(|r| r.folder.as_deref())
        .or_else(|| {
            settings
                .category_destinations
                .get(&name.to_lowercase())
                .map(PathBuf::as_path)
        })
        .unwrap_or(&settings.default_destination)
}
pub fn category_for<'a>(settings: &'a Settings, filename: &Path) -> &'a str {
    rusty_engine::categories::classify(filename, &settings.configuration.categories)
        .map(|r| r.name.as_str())
        .unwrap_or("General")
}
pub fn suggested_path(settings: &Settings, filename: &str, category: &str) -> PathBuf {
    let name = if category.eq_ignore_ascii_case("Automatic") {
        category_for(settings, Path::new(filename))
    } else {
        category
    };
    folder_for(settings, name).join(filename)
}
pub fn remember_folder(settings: &mut Settings, category: &str, destination: &Path) {
    let category = if category.eq_ignore_ascii_case("Automatic") {
        category_for(settings, destination).to_owned()
    } else {
        category.to_owned()
    };
    if let Some(rule) = settings
        .configuration
        .categories
        .iter_mut()
        .find(|r| r.name.eq_ignore_ascii_case(&category))
    {
        if !rule.remember {
            return;
        }
        if let Some(parent) = destination.parent() {
            rule.folder = Some(parent.to_owned());
            if rule.name == "General" {
                settings.default_destination = parent.to_owned();
            }
        }
    }
    if let Some(parent) = destination.parent() {
        settings
            .category_destinations
            .insert(category.trim().to_lowercase(), parent.to_owned());
    }
}

#[cfg(test)]
#[path = "download_form_tests.rs"]
mod tests;

/// A browser-provided name is never interpreted as a path.
pub fn filename_hint(address: &str, hint: Option<&str>) -> Result<String, JobError> {
    if let Some(hint) = hint {
        let basename = hint.rsplit(['/', '\\']).next().unwrap_or("");
        let name: String = basename
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
        if !name.is_empty()
            && name.encode_utf16().count() <= 255
            && validate_request(address, PathBuf::from(name), ReplacementPolicy::Deny).is_ok()
        {
            return Ok(name.to_owned());
        }
    }
    suggested_filename(address)
}
