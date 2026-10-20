//! Bounded capture policy shared by native admission and browser clients.
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowserKind {
    Chrome,
    Brave,
    Edge,
    Firefox,
}
impl BrowserKind {
    pub const ALL: [Self; 4] = [Self::Chrome, Self::Brave, Self::Edge, Self::Firefox];
    pub fn key(self) -> &'static str {
        match self {
            Self::Chrome => "chrome",
            Self::Brave => "brave",
            Self::Edge => "edge",
            Self::Firefox => "firefox",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Chrome => "Google Chrome",
            Self::Brave => "Brave Browser",
            Self::Edge => "Microsoft Edge",
            Self::Firefox => "Mozilla Firefox",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|b| b.key() == s)
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ModifierKeys {
    pub prevent: u8,
    pub force: u8,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MenuPolicy {
    pub links: bool,
    pub selection: bool,
    pub media: bool,
}
impl Default for MenuPolicy {
    fn default() -> Self {
        Self {
            links: true,
            selection: false,
            media: false,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PanelPolicy {
    pub enabled: bool,
    pub video: bool,
    pub audio: bool,
    pub hover_only: bool,
}
impl Default for PanelPolicy {
    fn default() -> Self {
        Self {
            enabled: false,
            video: true,
            audio: true,
            hover_only: true,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CapturePolicy {
    pub revision: u64,
    pub browsers: Vec<BrowserKind>,
    pub extensions: Vec<String>,
    pub excluded_sites: Vec<String>,
    pub excluded_addresses: Vec<String>,
    pub keys: ModifierKeys,
    pub menus: MenuPolicy,
    pub panels: PanelPolicy,
}
impl Default for CapturePolicy {
    fn default() -> Self {
        Self {
            revision: 1,
            browsers: BrowserKind::ALL.to_vec(),
            extensions: extensions(DEFAULT_EXTENSIONS).expect("factory extensions"),
            excluded_sites: vec![
                "*.update.microsoft.com".into(),
                "download.windowsupdate.com".into(),
                "*.download.windowsupdate.com".into(),
                "*.voice2page.com".into(),
            ],
            excluded_addresses: vec![],
            keys: Default::default(),
            menus: Default::default(),
            panels: Default::default(),
        }
    }
}
pub const DEFAULT_EXTENSIONS:&str="3GP 7Z AAC ACE AIF APK ARJ ASF AVI BIN BZ2 CAB COM DOC DOCX EPUB EXE FLAC FLV GZ GZIP IMG ISO LZH M4A M4V MD MKV MOV MP3 MP4 MPA MPE MPEG MPG MSI MSIX MSIXBUNDLE APPX APPXBUNDLE MSU OGG OGV ODT ODS ODP OPUS PDF PPT PPTX QT R0 R1 RA RAR RM RMVB RTF SEA SIT SITX TAR TGZ TIFF TIF TXT WAV WEBM WMA WMV XLS XLSX XZ Z ZIP ZST";
pub fn extensions(value: &str) -> Result<Vec<String>, String> {
    let mut out = BTreeSet::new();
    for token in value.split_whitespace() {
        let token = token.trim_start_matches('.').to_ascii_lowercase();
        if token.is_empty() || token.len() > 24 || !token.bytes().all(|c| c.is_ascii_alphanumeric())
        {
            return Err(
                "File types must be extension names separated by spaces (for example ZIP MP4 PDF)."
                    .into(),
            );
        }
        out.insert(token);
    }
    if out.len() > 512 {
        return Err("Too many file types (maximum 512).".into());
    }
    Ok(out.into_iter().collect())
}
pub fn site_patterns(value: &str) -> Result<Vec<String>, String> {
    let mut out = BTreeSet::new();
    for token in value.split_whitespace() {
        let token = token.to_ascii_lowercase();
        let host = token.strip_prefix("*.").unwrap_or(&token);
        if host.is_empty()
            || host.len() > 253
            || host.contains(['/', ':', '*', '?', '#', '@'])
            || host.split('.').any(|s| {
                s.is_empty()
                    || s.starts_with('-')
                    || s.ends_with('-')
                    || !s.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-')
            })
        {
            return Err("Use host names or *.example.com patterns, separated by spaces.".into());
        }
        out.insert(token);
    }
    if out.len() > 256 {
        return Err("Too many excluded sites (maximum 256).".into());
    }
    Ok(out.into_iter().collect())
}
pub fn address_pattern(value: &str) -> Result<String, String> {
    let value = value.trim();
    let prefix = value.ends_with('*');
    let raw = if prefix {
        &value[..value.len() - 1]
    } else {
        value
    };
    let url = url::Url::parse(raw)
        .map_err(|_| "Enter an HTTP(S) address, optionally ending with *.".to_owned())?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || raw.contains('*')
        || value.len() > 8192
        || value.chars().any(char::is_control)
    {
        return Err(
            "Enter a public HTTP(S) address without credentials or embedded wildcards.".into(),
        );
    }
    let mut url = url;
    url.set_fragment(None);
    Ok(format!("{}{}", url, if prefix { "*" } else { "" }))
}
impl CapturePolicy {
    pub fn validate(&self) -> Result<(), String> {
        if self.browsers.len() > 4
            || self.browsers.iter().copied().collect::<BTreeSet<_>>().len() != self.browsers.len()
        {
            return Err("Invalid browser selection.".into());
        }
        extensions(&self.extensions.join(" "))?;
        site_patterns(&self.excluded_sites.join(" "))?;
        if self.excluded_addresses.len() > 256 {
            return Err("Too many excluded addresses (maximum 256).".into());
        }
        for value in &self.excluded_addresses {
            address_pattern(value)?;
        }
        if self.keys.prevent > 7 || self.keys.force > 7 {
            return Err("Invalid modifier keys.".into());
        }
        Ok(())
    }
    pub fn excluded(&self, value: &str) -> bool {
        let Ok(mut url) = url::Url::parse(value) else {
            return true;
        };
        url.set_fragment(None);
        if !matches!(url.scheme(), "http" | "https")
            || !url.username().is_empty()
            || url.password().is_some()
        {
            return true;
        }
        let host = url.host_str().unwrap_or("").to_ascii_lowercase();
        self.excluded_sites.iter().any(|s| {
            if let Some(suffix) = s.strip_prefix("*.") {
                host.len() > suffix.len() && host.ends_with(&format!(".{suffix}"))
            } else {
                host == *s
            }
        }) || self.excluded_addresses.iter().any(|s| {
            if let Some(prefix) = s.strip_suffix('*') {
                url.as_str().starts_with(prefix)
            } else {
                url.as_str() == s
            }
        })
    }
    pub fn allows(
        &self,
        browser: BrowserKind,
        url: &str,
        filename: Option<&str>,
        forced: bool,
    ) -> bool {
        if !self.browsers.contains(&browser) || self.excluded(url) {
            return false;
        }
        let name = filename
            .map(str::to_owned)
            .or_else(|| {
                url::Url::parse(url)
                    .ok()
                    .and_then(|u| u.path_segments()?.next_back().map(str::to_owned))
            })
            .unwrap_or_default();
        forced
            || name
                .rsplit_once('.')
                .is_some_and(|(_, e)| self.extensions.iter().any(|s| s.eq_ignore_ascii_case(e)))
    }
}

impl Default for BrowserKind {
    fn default() -> Self {
        Self::Chrome
    }
}
