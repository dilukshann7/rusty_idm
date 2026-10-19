use serde::{Deserialize, Serialize};
use std::{
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    pub schema_version: u32,
    pub default_destination: PathBuf,
    #[serde(default)]
    pub category_destinations: std::collections::BTreeMap<String, PathBuf>,
    #[serde(default)]
    pub duplicate_choice: Option<crate::duplicates::DuplicateChoice>,
    #[serde(default = "show_completion_by_default")]
    pub show_completion_dialog: bool,
    #[serde(default)]
    pub double_click_action: DoubleClickAction,
}
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DoubleClickAction {
    Open,
    OpenFolder,
    #[default]
    Properties,
}
fn show_completion_by_default() -> bool {
    true
}
#[derive(Debug)]
pub struct SettingsError(pub String);
impl std::fmt::Display for SettingsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for SettingsError {}
fn path() -> Result<PathBuf, SettingsError> {
    std::env::var_os("LOCALAPPDATA")
        .map(|p| PathBuf::from(p).join("RustyIDM/settings.json"))
        .ok_or_else(|| SettingsError("Windows local application data is unavailable.".into()))
}
pub fn load() -> Result<Settings, SettingsError> {
    let path = path()?;
    match std::fs::symlink_metadata(&path) {
        Ok(_) => load_from(&path),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => defaults(),
        Err(_) => Err(SettingsError(
            "Cannot inspect settings. The saved file has been preserved.".into(),
        )),
    }
}
pub fn save(settings: &Settings) -> Result<(), SettingsError> {
    save_to(&path()?, settings)
}
pub fn load_from(path: &Path) -> Result<Settings, SettingsError> {
    let data = std::fs::read(path).map_err(|_| {
        SettingsError(
            "Cannot read settings. Using a temporary default does not change the saved file."
                .into(),
        )
    })?;
    if data.len() > 64 * 1024 {
        return Err(SettingsError("The settings file is too large.".into()));
    }
    let settings: Settings = serde_json::from_slice(&data).map_err(|_| {
        SettingsError("The settings file is malformed. It has been preserved.".into())
    })?;
    validate(&settings)?;
    Ok(settings)
}
fn validate(settings: &Settings) -> Result<(), SettingsError> {
    if settings.schema_version != 1 {
        return Err(SettingsError(
            "Unsupported settings version. The saved file has been preserved.".into(),
        ));
    }
    if !settings.default_destination.is_absolute() || !settings.default_destination.is_dir() {
        return Err(SettingsError(
            "The default save folder is unavailable. Select an existing folder in Options.".into(),
        ));
    }
    Ok(())
}
pub fn save_to(path: &Path, settings: &Settings) -> Result<(), SettingsError> {
    validate(settings)?;
    let parent = path
        .parent()
        .ok_or_else(|| SettingsError("Invalid settings path.".into()))?;
    std::fs::create_dir_all(parent)
        .map_err(|_| SettingsError("Cannot create the settings folder.".into()))?;
    let data = serde_json::to_vec_pretty(settings)
        .map_err(|_| SettingsError("Cannot serialize settings.".into()))?;
    if data.len() > 64 * 1024 {
        return Err(SettingsError("There are too many remembered category folders to save. The previous settings were preserved.".into()));
    }
    static NEXT: AtomicU64 = AtomicU64::new(1);
    let mut reserved = None;
    for _ in 0..128 {
        let temp = parent.join(format!(
            "settings-{}-{}.tmp",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        match std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temp)
        {
            Ok(file) => {
                reserved = Some((temp, file));
                break;
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(_) => {
                return Err(SettingsError(
                    "Cannot write settings. The previous settings were preserved.".into(),
                ))
            }
        }
    }
    let (temp, mut file) = reserved
        .ok_or_else(|| SettingsError("Cannot reserve a settings temporary file.".into()))?;
    let result = (|| {
        file.write_all(&data)?;
        file.sync_all()?;
        drop(file);
        atomic_replace(&temp, path)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    result.map_err(|_| {
        SettingsError("Saving settings failed. The previous settings were preserved.".into())
    })
}
#[cfg(windows)]
fn atomic_replace(source: &Path, target: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows::{
        core::PCWSTR,
        Win32::Storage::FileSystem::{
            MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
        },
    };
    let source: Vec<_> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let target: Vec<_> = target.as_os_str().encode_wide().chain(Some(0)).collect();
    unsafe {
        MoveFileExW(
            PCWSTR(source.as_ptr()),
            PCWSTR(target.as_ptr()),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    }
    .map_err(std::io::Error::from)
}
#[cfg(not(windows))]
fn atomic_replace(source: &Path, target: &Path) -> std::io::Result<()> {
    std::fs::rename(source, target)
}
#[cfg(windows)]
pub fn defaults() -> Result<Settings, SettingsError> {
    use std::os::windows::ffi::OsStringExt;
    use windows::Win32::{
        System::Com::CoTaskMemFree,
        UI::Shell::{FOLDERID_Downloads, SHGetKnownFolderPath, KF_FLAG_DEFAULT},
    };
    unsafe {
        let raw = SHGetKnownFolderPath(&FOLDERID_Downloads, KF_FLAG_DEFAULT, None)
            .map_err(|_| SettingsError("Cannot locate the Windows Downloads folder.".into()))?;
        let folder = std::ffi::OsString::from_wide(raw.as_wide());
        CoTaskMemFree(Some(raw.0.cast()));
        Ok(Settings {
            schema_version: 1,
            category_destinations: Default::default(),
            duplicate_choice: None,
            show_completion_dialog: true,
            double_click_action: DoubleClickAction::default(),
            default_destination: PathBuf::from(folder),
        })
    }
}
#[cfg(not(windows))]
pub fn defaults() -> Result<Settings, SettingsError> {
    Err(SettingsError(
        "The Windows Downloads known folder requires Windows.".into(),
    ))
}

#[cfg(test)]
#[path = "settings_tests.rs"]
mod tests;
