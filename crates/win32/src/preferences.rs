//! Configuration values independent of native controls; all drafts validate here.
use rusty_browser_bridge::policy::CapturePolicy;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ProgressPreferences {
    pub show: bool,
    pub foreground: bool,
    pub details: bool,
    pub close_when_finished: bool,
}
impl Default for ProgressPreferences {
    fn default() -> Self {
        Self {
            show: true,
            foreground: true,
            details: false,
            close_when_finished: true,
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ScannerPreferences {
    pub enabled: bool,
    pub executable: PathBuf,
    pub arguments: Vec<String>,
}
impl ScannerPreferences {
    pub fn validate(&self) -> Result<(), String> {
        if !self.enabled {
            return Ok(());
        }
        if !self.executable.is_absolute() || !self.executable.is_file() {
            return Err("Choose an existing absolute scanner executable.".into());
        }
        let extension = self
            .executable
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        if !extension.eq_ignore_ascii_case("exe") {
            return Err("Choose a scanner .exe program, not a shell script.".into());
        }
        if self.arguments.len() > 64
            || self
                .arguments
                .iter()
                .any(|s| s.len() > 4096 || s.contains(['\0', '\r', '\n']))
            || !self.arguments.iter().any(|s| s == "{file}")
        {
            return Err(
                "Scanner arguments need a standalone {file} argument; enter one argument per line."
                    .into(),
            );
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserRegistration {
    pub browser: rusty_browser_bridge::policy::BrowserKind,
    pub extension_id: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Configuration {
    #[serde(skip)]
    pub registrations: Vec<BrowserRegistration>,
    #[serde(skip)]
    pub renamed_categories: Vec<(String, String)>,
    #[serde(default)]
    pub browser_executables:
        std::collections::BTreeMap<rusty_browser_bridge::policy::BrowserKind, PathBuf>,
    pub revision: u64,
    pub startup: Option<bool>,
    pub clipboard: bool,
    #[serde(with = "saved_capture")]
    pub capture: CapturePolicy,
    pub categories: Vec<rusty_engine::categories::CategoryRule>,
    pub temporary_directory: Option<PathBuf>,
    pub server_creation_time: bool,
    pub show_start_dialog: bool,
    pub queue_only: bool,
    pub immediate_info: bool,
    pub choose_queue_later: bool,
    pub choose_queue_batch: bool,
    pub ignore_modified_time: bool,
    pub user_agent: String,
    pub progress: ProgressPreferences,
    pub scanner: ScannerPreferences,
}
impl Default for Configuration {
    fn default() -> Self {
        Self {
            registrations: vec![],
            renamed_categories: vec![],
            browser_executables: Default::default(),
            revision: 1,
            startup: None,
            clipboard: false,
            capture: Default::default(),
            categories: rusty_engine::categories::rules(),
            temporary_directory: None,
            server_creation_time: false,
            show_start_dialog: true,
            queue_only: false,
            immediate_info: false,
            choose_queue_later: false,
            choose_queue_batch: true,
            ignore_modified_time: false,
            user_agent: "RustyDownloadManager/0.1.0".into(),
            progress: Default::default(),
            scanner: Default::default(),
        }
    }
}
impl Configuration {
    pub fn validate(&self) -> Result<(), String> {
        self.capture.validate()?;
        rusty_engine::categories::validate_rules(&self.categories)?;
        self.download_options(true).validate()?;
        self.scanner.validate()?;
        for rule in &self.categories {
            if rule
                .folder
                .as_ref()
                .is_some_and(|p| !p.is_absolute() || !p.is_dir())
            {
                return Err(format!(
                    "Choose an existing absolute folder for {}.",
                    rule.name
                ));
            }
        }
        if self
            .temporary_directory
            .as_ref()
            .is_some_and(|p| !p.is_dir())
        {
            return Err("Choose an existing temporary folder.".into());
        }
        Ok(())
    }
    pub fn temporary_folder(&self) -> Option<PathBuf> {
        self.temporary_directory.clone().or_else(|| {
            std::env::var_os("LOCALAPPDATA").map(|p| PathBuf::from(p).join("RustyIDM/partials"))
        })
    }
    pub fn download_options(&self, manual: bool) -> rusty_engine::DownloadOptions {
        rusty_engine::DownloadOptions {
            user_agent: if manual {
                self.user_agent.clone()
            } else {
                "RustyDownloadManager/0.1.0".into()
            },
            temporary_directory: self.temporary_folder(),
            server_creation_time: self.server_creation_time,
            ignore_modified_time: self.ignore_modified_time,
            confirmation_required: false,
        }
    }
}
mod saved_capture {
    use super::*;
    use serde::{de::Error as _, ser::Error as _, Deserializer, Serializer};
    #[derive(Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Saved {
        policy: CapturePolicy,
        #[serde(default)]
        private_addresses: Vec<u8>,
    }
    pub fn serialize<S: Serializer>(policy: &CapturePolicy, s: S) -> Result<S::Ok, S::Error> {
        let mut public = policy.clone();
        let addresses = std::mem::take(&mut public.excluded_addresses);
        let private_addresses = if addresses.is_empty() {
            vec![]
        } else {
            rusty_browser_bridge::private::crypt(
                &serde_json::to_vec(&addresses).map_err(S::Error::custom)?,
                true,
            )
            .map_err(S::Error::custom)?
        };
        Saved {
            policy: public,
            private_addresses,
        }
        .serialize(s)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<CapturePolicy, D::Error> {
        let mut saved = Saved::deserialize(d)?;
        if !saved.policy.excluded_addresses.is_empty() {
            return Err(D::Error::custom("Private address rules must be encrypted."));
        }
        if !saved.private_addresses.is_empty() {
            let bytes = rusty_browser_bridge::private::crypt(&saved.private_addresses, false)
                .map_err(D::Error::custom)?;
            saved.policy.excluded_addresses =
                serde_json::from_slice(&bytes).map_err(D::Error::custom)?;
        }
        saved.policy.validate().map_err(D::Error::custom)?;
        Ok(saved.policy)
    }
}
