use super::*;
use ring::{
    digest,
    rand::{SecureRandom, SystemRandom},
};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

pub(crate) struct ProjectStore {
    pub root: Option<PathBuf>,
}
pub(crate) fn new_id() -> Result<ProjectId, String> {
    let mut bytes = [0u8; 16];
    SystemRandom::new()
        .fill(&mut bytes)
        .map_err(|_| "Cannot generate a project identity.")?;
    Ok(ProjectId(hex(&bytes)))
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
pub(crate) fn hash(value: &[u8]) -> String {
    hex(digest::digest(&digest::SHA256, value).as_ref())
}
pub(crate) fn read_document(path: &Path, limit: u64) -> Result<Vec<u8>, String> {
    use std::io::Read;
    guarded_directory(path.parent().ok_or("Invalid original path.")?)?;
    reject_link(path)?;
    let file = std::fs::File::open(path).map_err(|_| "Cannot read frozen scan original.")?;
    let mut bytes = Vec::new();
    file.take(limit.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|_| "Cannot read frozen scan original.")?;
    if bytes.len() as u64 > limit {
        return Err("Frozen scan original exceeds its document limit.".into());
    }
    Ok(bytes)
}
pub(crate) fn safe_id(id: &ProjectId) -> bool {
    id.0.len() == 32 && id.0.bytes().all(|c| c.is_ascii_hexdigit())
}
impl ProjectStore {
    pub fn new(durable: bool) -> Self {
        Self {
            root: durable
                .then(|| {
                    std::env::var_os("LOCALAPPDATA")
                        .map(|p| PathBuf::from(p).join("RustyIDM").join("grabber"))
                })
                .flatten(),
        }
    }
    pub fn directory(&self, id: &ProjectId) -> Result<PathBuf, String> {
        if !safe_id(id) {
            return Err("Invalid project identity.".into());
        }
        let root = self
            .root
            .as_ref()
            .ok_or("Project storage is unavailable.")?;
        let path = root.join(&id.0);
        guarded_directory(&path)?;
        Ok(path)
    }
    pub fn raw(&self, p: &Project, item: ItemId) -> Result<PathBuf, String> {
        let dir = self
            .directory(&p.id)?
            .join(format!("scan-{}", p.generation));
        guarded_directory(&dir)?;
        Ok(dir.join(format!("{}.raw", item.0)))
    }
    pub fn save(&self, p: &Project) -> Result<(), String> {
        if self.root.is_none() {
            return Err("Saved projects require local persistent storage.".into());
        }
        let mut plain = serde_json::to_vec(p).map_err(|_| "Cannot encode the project.")?;
        if plain.len() > 16 * 1024 * 1024 {
            return Err(
                "Project manifest reached its 16 MiB limit. Narrow the crawl filters.".into(),
            );
        }
        let encrypted = crate::storage::crypt(&plain, true);
        plain.fill(0);
        let encrypted = encrypted?;
        if encrypted.len() > 16 * 1024 * 1024 {
            return Err("Encrypted project manifest exceeds 16 MiB.".into());
        }
        let path = self.directory(&p.id)?.join("project.v1.dpapi");
        reject_link(&path)?;
        crate::storage::atomic_write(&path, &encrypted)
    }
    pub fn load(&self) -> (BTreeMap<ProjectId, Project>, Option<String>) {
        let mut projects = BTreeMap::new();
        let mut errors = vec![];
        let Some(root) = &self.root else {
            return (projects, None);
        };
        if !root.exists() {
            return (projects, None);
        }
        if guarded_directory(root).is_err() {
            return (
                projects,
                Some("Grabber storage contains an unsafe directory.".into()),
            );
        }
        let Ok(dirs) = std::fs::read_dir(root) else {
            return (projects, Some("Cannot read saved grabber projects.".into()));
        };
        for dir in dirs.flatten() {
            let id = ProjectId(dir.file_name().to_string_lossy().into());
            if !safe_id(&id) {
                continue;
            }
            let result: Result<Project, String> = (|| {
                let folder = self.directory(&id)?;
                let path = folder.join("project.v1.dpapi");
                reject_link(&path)?;
                let info = std::fs::metadata(&path).map_err(|_| "Cannot read project metadata.")?;
                if info.len() > 16 * 1024 * 1024 {
                    return Err("Project is too large.".into());
                }
                let encrypted = std::fs::read(path).map_err(|_| "Cannot read project.")?;
                let mut plain = crate::storage::crypt(&encrypted, false)
                    .map_err(|_| "Cannot decrypt project.")?;
                let decoded = serde_json::from_slice::<Project>(&plain);
                plain.fill(0);
                let mut p = decoded.map_err(|_| "Project is corrupt or incompatible.")?;
                if p.version != 1
                    || p.id != id
                    || p.items.len() > 100_000
                    || p.generation == 0
                    || p.items.iter().any(|(id, i)| {
                        id != &i.id
                            || public_url(&i.url).is_err()
                            || public_url(&i.final_url).is_err()
                            || i.original
                                .as_ref()
                                .is_some_and(|name| name != &format!("{}.raw", id.0))
                    })
                {
                    return Err("Project contains invalid data.".into());
                }
                p.config
                    .validate()
                    .map_err(|_| "Project configuration is invalid.")?;
                if let Some(super::model::ProjectPending::Rescan(config)) = &mut p.pending {
                    config
                        .validate()
                        .map_err(|_| "Pending project configuration is invalid.")?;
                }
                let output_root = p.config.destination.join(format!("rusty-site-{}", p.id.0));
                if p.offline.as_ref().is_some_and(|path| {
                    !path.starts_with(&output_root)
                        || path.file_name().is_none_or(|name| name != "index.html")
                        || path
                            .components()
                            .any(|c| matches!(c, std::path::Component::ParentDir))
                }) {
                    return Err("Saved offline output path is invalid.".into());
                }
                if p.next_item == 0 || p.items.keys().any(|id| id.0 == 0 || id.0 >= p.next_item) {
                    return Err("Saved project item identities are invalid.".into());
                }
                p.frontier.retain(|id| p.items.contains_key(id));
                for item in p.items.values_mut() {
                    if item.status == ItemStatus::Fetching {
                        item.status = ItemStatus::Discovered;
                        if !p.frontier.contains(&item.id) {
                            p.frontier.push_back(item.id);
                        }
                    }
                    if matches!(item.status, ItemStatus::Queued | ItemStatus::Downloading) {
                        item.status = ItemStatus::Ready;
                    }
                    if let Some(name) = &item.original {
                        let path = folder.join(format!("scan-{}", p.generation)).join(name);
                        let valid = guarded_directory(path.parent().expect("original parent"))
                            .is_ok()
                            && reject_link(&path).is_ok()
                            && std::fs::metadata(&path)
                                .is_ok_and(|m| m.is_file() && m.len() <= p.config.document_limit)
                            && read_document(&path, p.config.document_limit).is_ok_and(|bytes| {
                                item.original_hash.as_deref() == Some(hash(&bytes).as_str())
                            });
                        if !valid {
                            item.status = ItemStatus::Missing;
                            item.message =
                                "Cached original is missing. Rescan this project.".into();
                        }
                    }
                }
                if matches!(
                    p.state,
                    ProjectState::Scanning | ProjectState::Downloading | ProjectState::Exporting
                ) {
                    p.state = ProjectState::Paused;
                    p.message = "Interrupted project restored. Resume manually.".into();
                }
                if p.pending.is_some() {
                    p.state = ProjectState::Paused;
                    p.message =
                        "An interrupted rescan or deletion is saved. Resume to complete it.".into();
                }
                Ok(p)
            })();
            match result {
                Ok(p) => {
                    projects.insert(id, p);
                }
                Err(e) => errors.push(format!(
                    "{}: {e} Original project files were preserved.",
                    id.0
                )),
            }
        }
        (projects, (!errors.is_empty()).then(|| errors.join("\n")))
    }
    pub fn remove(&self, id: &ProjectId) -> Result<(), String> {
        let folder = self.directory(id)?;
        // Enumerate and reject reparse points before removing any owned cache.
        fn inspect(path: &Path) -> Result<(), String> {
            reject_link(path)?;
            if path.is_dir() {
                for entry in std::fs::read_dir(path).map_err(|_| "Cannot inspect project cache.")? {
                    inspect(
                        &entry
                            .map_err(|_| "Cannot inspect project cache entry.")?
                            .path(),
                    )?;
                }
            }
            Ok(())
        }
        inspect(&folder)?;
        std::fs::remove_dir_all(folder)
            .map_err(|_| "Cannot remove project storage. Exported files were preserved.".into())
    }
}
pub(crate) fn reject_link(path: &Path) -> Result<(), String> {
    match std::fs::symlink_metadata(path) {
        Ok(meta) => {
            #[cfg(windows)]
            {
                use std::os::windows::fs::MetadataExt;
                if meta.file_attributes() & 0x400 != 0 {
                    return Err("Reparse points are not allowed in grabber paths.".into());
                }
            }
            if meta.file_type().is_symlink() {
                return Err("Symbolic links are not allowed in grabber paths.".into());
            }
            Ok(())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err("Cannot inspect a grabber path.".into()),
    }
}
pub(crate) fn guarded_directory(path: &Path) -> Result<(), String> {
    if !path.is_absolute()
        || path.components().any(|c| {
            matches!(
                c,
                std::path::Component::ParentDir | std::path::Component::CurDir
            )
        })
    {
        return Err(
            "Grabber paths must be absolute and cannot contain traversal components.".into(),
        );
    }
    for part in path.ancestors() {
        reject_link(part)?;
    }
    std::fs::create_dir_all(path).map_err(|_| "Cannot create a grabber directory.")?;
    for part in path.ancestors() {
        reject_link(part)?;
    }
    Ok(())
}
pub(crate) fn write_new(path: &Path, bytes: &[u8]) -> Result<(), String> {
    use std::io::Write;
    guarded_directory(path.parent().ok_or("Invalid output path.")?)?;
    reject_link(path)?;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|_| "Cannot create a new owned grabber file; existing files are preserved.")?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|_| "Cannot write or sync grabber output.".into())
}
