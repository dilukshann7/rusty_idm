use serde::Deserialize;
use std::{
    io::{self, Read},
    path::{Path, PathBuf},
};
pub type HostError = io::Error;
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostConfig {
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
            || config.extension_id.len() != 32
            || !config
                .extension_id
                .bytes()
                .all(|b| (b'a'..=b'p').contains(&b))
        {
            return Err(io::Error::other(
                "Invalid installed host paths or extension ID",
            ));
        }
        Ok(config)
    }
    pub fn validate_origin(&self, origin: &str) -> Result<(), HostError> {
        if origin != format!("chrome-extension://{}/", self.extension_id) {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Unexpected native caller",
            ));
        }
        Ok(())
    }
}
