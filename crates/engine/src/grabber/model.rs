use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, VecDeque},
    path::PathBuf,
};
use url::Url;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ProjectId(pub String);
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ItemId(pub u64);

/// Durable idempotency key and per-request redirect boundary.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectMember {
    pub project: ProjectId,
    pub generation: u64,
    pub item: ItemId,
    pub allowed_hosts: Vec<String>,
    pub mirror: bool,
    pub max_redirects: usize,
    pub exclude_urls: Vec<String>,
}
impl ProjectMember {
    pub(crate) fn same_item(&self, other: &Self) -> bool {
        self.project == other.project
            && self.generation == other.generation
            && self.item == other.item
    }
    pub(crate) fn validate(&self) -> Result<(), String> {
        if !super::store::safe_id(&self.project)
            || self.generation == 0
            || self.item.0 == 0
            || self.allowed_hosts.is_empty()
            || self.allowed_hosts.len() > 128
            || self.max_redirects > 10
            || self.exclude_urls.len() > 128
            || self.exclude_urls.iter().any(|p| p.len() > 1024)
        {
            return Err("Invalid stored project membership.".into());
        }
        for host in &self.allowed_hosts {
            let name = host.strip_prefix("*.").unwrap_or(host);
            if name.is_empty()
                || name.contains(['/', '@', '?', '#', '*'])
                || url::Host::parse(name).is_err()
            {
                return Err("Invalid stored project host scope.".into());
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GrabberMode {
    CollectFiles,
    OfflineWebsite,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectConfig {
    #[serde(default)]
    pub transfer_options: crate::DownloadOptions,
    pub name: String,
    pub start_url: String,
    pub destination: PathBuf,
    pub mode: GrabberMode,
    pub page_hosts: Vec<String>,
    pub asset_hosts: Vec<String>,
    pub include_urls: Vec<String>,
    pub exclude_urls: Vec<String>,
    pub extensions: Vec<String>,
    pub mime_types: Vec<String>,
    pub max_depth: u16,
    pub max_pages: usize,
    pub max_results: usize,
    pub scan_workers: usize,
    pub host_delay_ms: u64,
    pub max_redirects: usize,
    pub document_limit: u64,
    pub cache_limit: u64,
    pub respect_robots: bool,
}
impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            transfer_options: Default::default(),
            name: "New project".into(),
            start_url: String::new(),
            destination: PathBuf::new(),
            mode: GrabberMode::CollectFiles,
            page_hosts: vec![],
            asset_hosts: vec![],
            include_urls: vec![],
            exclude_urls: vec![],
            extensions: vec![],
            mime_types: vec![],
            max_depth: 2,
            max_pages: 1000,
            max_results: 10_000,
            scan_workers: 2,
            host_delay_ms: 500,
            max_redirects: 10,
            document_limit: 8 * 1024 * 1024,
            cache_limit: 256 * 1024 * 1024,
            respect_robots: true,
        }
    }
}
impl ProjectConfig {
    pub fn validate(&mut self) -> Result<(), String> {
        self.transfer_options.validate()?;
        let seed = public_url(&self.start_url)?;
        self.start_url = seed.as_str().into();
        self.name = self.name.trim().into();
        if self.name.is_empty() || self.name.len() > 128 {
            return Err("Use a project name of 1–128 bytes.".into());
        }
        crate::validate_request(
            &self.start_url,
            self.destination.join("grabber-check.bin"),
            crate::ReplacementPolicy::Deny,
        )
        .map_err(|e| e.message)?;
        for list in [&mut self.page_hosts, &mut self.asset_hosts] {
            if list.is_empty() {
                list.push(seed.host_str().unwrap_or_default().into());
            }
            if list.len() > 128 {
                return Err("At most 128 allowed hosts per list.".into());
            }
            for host in list.iter_mut() {
                *host = host.trim().to_ascii_lowercase();
                let name = host.strip_prefix("*.").unwrap_or(host);
                if name.is_empty()
                    || name.contains(['/', '@', '?', '#', '*'])
                    || url::Host::parse(name).is_err()
                {
                    return Err("Allowed hosts must be hostnames or *.example.com patterns, without paths or ports.".into());
                }
                let wildcard = host.starts_with("*.");
                let normalized = url::Host::parse(name)
                    .map_err(|_| "Invalid allowed host.")?
                    .to_string();
                *host = if wildcard {
                    format!("*.{normalized}")
                } else {
                    normalized
                };
            }
        }
        if !host_allowed(&seed, &self.page_hosts) {
            return Err("The starting host must be in the page allowlist.".into());
        }
        if self.max_pages == 0
            || self.max_pages > 100_000
            || self.max_results == 0
            || self.max_results > 100_000
            || !(1..=2).contains(&self.scan_workers)
            || self.host_delay_ms < 500
            || self.host_delay_ms > 60_000
            || self.max_redirects > 10
            || self.document_limit == 0
            || self.document_limit > 64 * 1024 * 1024
            || self.cache_limit < self.document_limit
            || self.cache_limit > 4 * 1024 * 1024 * 1024u64
            || self.max_depth > 100
        {
            return Err("Invalid crawl limits: 1–100000 pages/results, 1–2 workers, 500–60000 ms delay, depth 0–100, at most 10 redirects, documents at most 64 MiB and cache at most 4096 MiB.".into());
        }
        for list in [
            &self.include_urls,
            &self.exclude_urls,
            &self.extensions,
            &self.mime_types,
        ] {
            if list.len() > 128
                || list
                    .iter()
                    .any(|s| s.len() > 1024 || s.contains(['\r', '\n', '\0']))
            {
                return Err("Filter lists support at most 128 entries of 1024 bytes.".into());
            }
        }
        if self.exclude_urls.iter().any(|p| glob(p, seed.as_str())) {
            return Err("The starting URL is excluded by the project filters.".into());
        }
        Ok(())
    }
    pub(crate) fn allows(&self, url: &Url, page: bool) -> bool {
        host_allowed(
            url,
            if page {
                &self.page_hosts
            } else {
                &self.asset_hosts
            },
        ) && !self.exclude_urls.iter().any(|p| glob(p, url.as_str()))
    }
    pub(crate) fn matches_file(&self, url: &Url, mime: &str) -> bool {
        let ext = url
            .path()
            .rsplit('/')
            .next()
            .unwrap_or_default()
            .rsplit_once('.')
            .map(|(_, v)| v.to_ascii_lowercase());
        (self.include_urls.is_empty() || self.include_urls.iter().any(|p| glob(p, url.as_str())))
            && (self.extensions.is_empty()
                || ext.as_ref().is_some_and(|e| {
                    self.extensions
                        .iter()
                        .any(|p| glob(&p.trim_start_matches('.').to_ascii_lowercase(), e))
                }))
            && (self.mime_types.is_empty()
                || (!mime.is_empty()
                    && self
                        .mime_types
                        .iter()
                        .any(|p| glob(&p.to_ascii_lowercase(), mime))))
    }
}

pub(crate) fn public_url(value: &str) -> Result<Url, String> {
    let mut url = Url::parse(value.trim()).map_err(|_| "Enter an absolute HTTP/HTTPS URL.")?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.as_str().len() > 8192
    {
        return Err(
            "Only HTTP/HTTPS URLs without login credentials, at most 8192 bytes, are supported."
                .into(),
        );
    }
    url.set_fragment(None);
    Ok(url)
}
pub(crate) fn host_allowed(url: &Url, hosts: &[String]) -> bool {
    let Some(host) = url.host_str() else {
        return false;
    };
    matches!(url.scheme(), "http" | "https")
        && url.username().is_empty()
        && url.password().is_none()
        && hosts.iter().any(|rule| match rule.strip_prefix("*.") {
            Some(suffix) => host.len() > suffix.len() + 1 && host.ends_with(&format!(".{suffix}")),
            None => host.eq_ignore_ascii_case(rule),
        })
}
/// Linear-space glob matching. '*' and '?' operate on UTF-8 bytes, not regexes.
pub(crate) fn glob(pattern: &str, value: &str) -> bool {
    let (p, v) = (pattern.as_bytes(), value.as_bytes());
    let (mut i, mut j, mut star, mut checkpoint) = (0, 0, None, 0);
    while j < v.len() {
        if i < p.len() && (p[i] == b'?' || p[i] == v[j]) {
            i += 1;
            j += 1;
        } else if i < p.len() && p[i] == b'*' {
            star = Some(i);
            i += 1;
            checkpoint = j;
        } else if let Some(s) = star {
            checkpoint += 1;
            j = checkpoint;
            i = s + 1;
        } else {
            return false;
        }
    }
    while i < p.len() && p[i] == b'*' {
        i += 1;
    }
    i == p.len()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItemKind {
    Unknown,
    Page,
    Stylesheet,
    Asset,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItemStatus {
    Discovered,
    Fetching,
    Ready,
    Excluded,
    Queued,
    Downloading,
    Complete,
    Failed,
    Missing,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrabberItem {
    pub id: ItemId,
    pub url: String,
    pub final_url: String,
    pub kind: ItemKind,
    pub depth: u16,
    pub selected: bool,
    pub eligible: bool,
    pub explicit_selection: bool,
    pub mime: String,
    pub size: Option<u64>,
    pub status: ItemStatus,
    pub message: String,
    pub dependencies: Vec<ItemId>,
    pub original: Option<String>,
    pub original_hash: Option<String>,
    pub entry: Option<crate::EntryId>,
    pub local: Option<PathBuf>,
    pub local_identity: Option<crate::DestinationIdentity>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectState {
    Idle,
    Scanning,
    Paused,
    Review,
    Downloading,
    Exporting,
    Complete,
    Failed,
}
#[derive(Clone, Debug)]
pub struct ProjectSnapshot {
    pub id: ProjectId,
    pub name: String,
    pub generation: u64,
    pub state: ProjectState,
    pub results: usize,
    pub selected: usize,
    pub completed: usize,
    pub message: String,
    pub destination: PathBuf,
    pub offline: Option<PathBuf>,
}
#[derive(Clone, Serialize, Deserialize)]
pub(crate) enum ProjectPending {
    Delete,
    Rescan(ProjectConfig),
}
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Project {
    pub version: u32,
    pub id: ProjectId,
    pub generation: u64,
    pub config: ProjectConfig,
    pub state: ProjectState,
    pub items: BTreeMap<ItemId, GrabberItem>,
    pub frontier: VecDeque<ItemId>,
    pub next_item: u64,
    pub cache_bytes: u64,
    pub message: String,
    pub publication: u64,
    pub offline: Option<PathBuf>,
    pub downloading: bool,
    pub limit_reached: bool,
    #[serde(default)]
    pub pending: Option<ProjectPending>,
}
impl Project {
    pub fn snapshot(&self) -> ProjectSnapshot {
        ProjectSnapshot {
            id: self.id.clone(),
            name: self.config.name.clone(),
            generation: self.generation,
            state: self.state,
            results: self.items.len(),
            selected: self.items.values().filter(|i| i.selected).count(),
            completed: self
                .items
                .values()
                .filter(|i| i.status == ItemStatus::Complete)
                .count(),
            message: self.message.clone(),
            destination: self.config.destination.clone(),
            offline: self.offline.clone(),
        }
    }
    pub fn member(&self, item: ItemId) -> ProjectMember {
        ProjectMember {
            project: self.id.clone(),
            generation: self.generation,
            item,
            allowed_hosts: if self
                .items
                .get(&item)
                .is_some_and(|i| i.kind == ItemKind::Page)
            {
                self.config.page_hosts.clone()
            } else {
                self.config.asset_hosts.clone()
            },
            mirror: self.config.mode == GrabberMode::OfflineWebsite,
            max_redirects: self.config.max_redirects,
            exclude_urls: self.config.exclude_urls.clone(),
        }
    }
}
