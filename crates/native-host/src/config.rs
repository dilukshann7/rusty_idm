use serde::Deserialize;
use std::{
    io::{self, Read},
    path::{Path, PathBuf},
};
pub type HostError = io::Error;
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostConfig {
    #[serde(default)]
    pub browser: rusty_browser_bridge::policy::BrowserKind,
    #[serde(default)]
    pub extension_ids: Vec<String>,
    pub extension_id: String,
    pub gui_exe: PathBuf,
    pub host_exe: PathBuf,
}
impl HostConfig {
    pub fn load(path: &Path) -> Result<Self, HostError> {
        if !path.is_absolute() {
            return Err(io::Error::other("Absolute host configuration required"));
        }
        let file = std::fs::File::open(path)?;
        let mut bytes = Vec::new();
        file.take(65537).read_to_end(&mut bytes)?;
        if bytes.len() > 65536 {
            return Err(io::Error::other("Host configuration too large"));
        }
        let config: Self = serde_json::from_slice(&bytes)
            .map_err(|_| io::Error::other("Invalid host configuration"))?;
        if !config.gui_exe.is_absolute()
            || !config.gui_exe.is_file()
            || !config.host_exe.is_absolute()
            || !config.host_exe.is_file()
            || !config.valid_id(&config.extension_id)
            || config.extension_ids.len() > 32
            || config.extension_ids.iter().any(|id| !config.valid_id(id))
        {
            return Err(io::Error::other(
                "Invalid installed host paths or extension ID",
            ));
        }
        Ok(config)
    }
    fn valid_id(&self, id: &str) -> bool {
        if self.browser == rusty_browser_bridge::policy::BrowserKind::Firefox {
            id.len() >= 3
                && id.len() <= 200
                && id
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"@._-{}".contains(&c))
        } else {
            id.len() == 32 && id.bytes().all(|c| (b'a'..=b'p').contains(&c))
        }
    }
    pub fn validate_origin(&self, origin: &str) -> Result<(), HostError> {
        let accepted = std::iter::once(&self.extension_id)
            .chain(self.extension_ids.iter())
            .any(|id| {
                if self.browser == rusty_browser_bridge::policy::BrowserKind::Firefox {
                    origin == id
                } else {
                    origin == format!("chrome-extension://{id}/")
                }
            });
        if !accepted {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Unexpected native caller",
            ));
        }
        Ok(())
    }
}
