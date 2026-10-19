use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DuplicateChoice {
    Numbered,
    Overwrite,
    #[default]
    Existing,
}

pub fn same_destination(a: &Path, b: &Path) -> bool {
    a.to_string_lossy().replace('/', "\\").to_lowercase()
        == b.to_string_lossy().replace('/', "\\").to_lowercase()
}

/// Suggestion only: the engine still reserves a unique partial and refuses
/// unexpected target replacement if a file appears after this check.
pub fn numbered_destination(base: &Path, reserved: &[PathBuf]) -> std::io::Result<PathBuf> {
    let parent = base.parent().unwrap_or(Path::new("."));
    let stem = base
        .file_stem()
        .unwrap_or_else(|| std::ffi::OsStr::new("download"));
    for number in 1..=100_000 {
        let mut filename = stem.to_os_string();
        filename.push(format!(" ({number})"));
        if let Some(extension) = base.extension() {
            filename.push(".");
            filename.push(extension);
        }
        let candidate = parent.join(filename);
        if reserved
            .iter()
            .any(|path| same_destination(path, &candidate))
        {
            continue;
        }
        match std::fs::symlink_metadata(&candidate) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(candidate),
            Ok(_) => continue,
            Err(error) => return Err(error),
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        "No free numbered filename was found.",
    ))
}

#[cfg(test)]
#[path = "duplicates_tests.rs"]
mod tests;
