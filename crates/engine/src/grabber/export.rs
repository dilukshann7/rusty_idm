use super::{
    document,
    store::{self, ProjectStore},
    *,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{Read, Write},
    path::{Path, PathBuf},
};
use tokio_util::sync::CancellationToken;
use url::Url;

pub(crate) struct Published {
    pub index: PathBuf,
    pub missing: usize,
}
pub(crate) fn filename(item: &GrabberItem) -> String {
    let url = Url::parse(&item.url).ok();
    let original = url
        .as_ref()
        .and_then(|u| u.path_segments())
        .and_then(|mut s| s.next_back())
        .unwrap_or("resource");
    let (stem, extension) = original.rsplit_once('.').unwrap_or((original, ""));
    let mut stem: String = stem
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
        .take(40)
        .collect();
    if stem.is_empty() {
        stem = "resource".into();
    }
    let extension = match item.kind {
        ItemKind::Page => "html".into(),
        ItemKind::Stylesheet => "css".into(),
        _ => {
            let filtered: String = extension
                .chars()
                .filter(|c| c.is_ascii_alphanumeric())
                .take(12)
                .collect();
            if filtered.is_empty() {
                match item.mime.as_str() {
                    "image/png" => "png",
                    "image/jpeg" => "jpg",
                    "image/svg+xml" => "svg",
                    "image/webp" => "webp",
                    "application/pdf" => "pdf",
                    "application/javascript" | "text/javascript" => "js",
                    "font/woff2" => "woff2",
                    _ => "bin",
                }
                .into()
            } else {
                filtered
            }
        }
    };
    // An alphabetic prefix also avoids Windows device basenames.
    format!(
        "r-{stem}-{}.{}",
        &store::hash(item.url.as_bytes())[..32],
        extension
    )
}
fn stopped(token: &CancellationToken) -> Result<(), String> {
    if token.is_cancelled() {
        Err("Export stopped. Earlier snapshots were preserved.".into())
    } else {
        Ok(())
    }
}
fn path_url(path: &str) -> String {
    path.bytes()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, b'/' | b'-' | b'_' | b'.') {
                (c as char).to_string()
            } else {
                format!("%{c:02X}")
            }
        })
        .collect()
}
pub(crate) fn publish(
    p: &Project,
    storage: &ProjectStore,
    token: &CancellationToken,
) -> Result<Published, String> {
    stopped(token)?;
    let root = p.config.destination.join(format!("rusty-site-{}", p.id.0));
    store::guarded_directory(&root)?;
    let directory = root.join(format!("snapshot-{}-{}", p.generation, p.publication));
    // A failed/interrupted publication never reuses a previously created folder.
    std::fs::create_dir(&directory).map_err(|_| {
        "Cannot reserve a fresh snapshot folder. Existing snapshots were preserved."
    })?;
    store::guarded_directory(&directory)?;
    let files = directory.join("files");
    std::fs::create_dir(&files).map_err(|_| "Cannot create snapshot files directory.")?;
    let mut mapping = BTreeMap::new();
    let mut successful = BTreeMap::new();
    let mut missing = 0;
    for item in p.items.values().filter(|i| i.selected && i.eligible) {
        stopped(token)?;
        if item.status != ItemStatus::Complete {
            missing += 1;
            continue;
        }
        let target = filename(item);
        if item.original.is_some() {
            let source = storage.raw(p, item.id)?;
            store::reject_link(&source)?;
            if !source.is_file() {
                missing += 1;
                continue;
            }
            successful.insert(item.id, (target, source, true));
        } else if let Some(source) = &item.local {
            store::reject_link(source)?;
            let identity = crate::inspect_destination(source).ok().flatten();
            if identity.is_none() || identity != item.local_identity {
                missing += 1;
                continue;
            }
            if copy_new(source, &files.join(&target), token).is_err() {
                missing += 1;
                continue;
            }
            successful.insert(item.id, (target, source.clone(), false));
        } else {
            missing += 1;
        }
    }
    for (id, (target, _, _)) in &successful {
        let item = &p.items[id];
        mapping
            .entry(item.url.clone())
            .or_insert_with(|| target.clone());
        mapping
            .entry(item.final_url.clone())
            .or_insert_with(|| target.clone());
    }
    // Parse all rewritten documents before publishing any of them. If one fails,
    // remove its mapping and regenerate the others with online fallback links.
    loop {
        let mut failed = vec![];
        for (id, (_, source, is_document)) in &successful {
            if !is_document {
                continue;
            }
            stopped(token)?;
            let item = &p.items[id];
            let result = (|| render(p, item, source, &mapping))();
            match result {
                Ok(_) => {}
                Err(_) => failed.push(*id),
            }
        }
        if failed.is_empty() {
            break;
        }
        for id in failed {
            successful.remove(&id);
            missing += 1;
            let item = &p.items[&id];
            mapping.remove(&item.url);
            mapping.remove(&item.final_url);
        }
    }
    let mut online = BTreeSet::new();
    for (id, (target, source, is_document)) in &successful {
        if !is_document {
            continue;
        }
        stopped(token)?;
        let (value, references) = render(p, &p.items[id], source, &mapping)?;
        online.extend(references);
        store::write_new(&files.join(target), value.as_bytes())?;
    }
    missing = missing.max(online.len());
    stopped(token)?;
    let seed = successful
        .get(&ItemId(1))
        .ok_or("The starting page could not be exported. Existing snapshots were preserved.")?;
    let entry = path_url(&format!("files/{}", seed.0));
    let html=format!("<!doctype html><html><head><meta charset=\"utf-8\"><meta http-equiv=\"refresh\" content=\"0;url={entry}\"><title>Rusty offline website</title></head><body><a href=\"{entry}\">Open saved website</a></body></html>");
    store::write_new(&directory.join("index.html"), html.as_bytes())?;
    Ok(Published {
        index: directory.join("index.html"),
        missing,
    })
}
fn copy_new(source: &Path, target: &Path, token: &CancellationToken) -> Result<(), String> {
    store::guarded_directory(source.parent().ok_or("Invalid source folder.")?)?;
    store::reject_link(source)?;
    let mut input = std::fs::File::open(source).map_err(|_| "Cannot read downloaded resource.")?;
    let mut output = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(target)
        .map_err(|_| "Cannot create snapshot resource.")?;
    let mut buffer = vec![0; 128 * 1024];
    loop {
        stopped(token)?;
        let n = input
            .read(&mut buffer)
            .map_err(|_| "Cannot read downloaded resource.")?;
        if n == 0 {
            break;
        }
        output
            .write_all(&buffer[..n])
            .map_err(|_| "Cannot write snapshot resource.")?;
    }
    output
        .sync_all()
        .map_err(|_| "Cannot sync snapshot resource.".into())
}

fn render(
    p: &Project,
    item: &GrabberItem,
    source: &Path,
    mapping: &BTreeMap<String, String>,
) -> Result<(String, BTreeSet<String>), String> {
    store::reject_link(source)?;
    let bytes = store::read_document(source, p.config.document_limit)?;
    if bytes.len() as u64 > p.config.document_limit {
        return Err("Cached original exceeds the configured document limit.".into());
    }
    if item.original_hash.as_deref() != Some(store::hash(&bytes).as_str()) {
        return Err("Frozen scan original changed. Rescan this project.".into());
    }
    let input = String::from_utf8(bytes).map_err(|_| "Cached original is not UTF-8.")?;
    let base = public_url(&item.final_url)?;
    let online = std::cell::RefCell::new(BTreeSet::new());
    let resolve = |value: &str, navigation: bool| -> String {
        if value.trim().starts_with('#') {
            return value.into();
        }
        let Ok(mut url) = base.join(value) else {
            return value.into();
        };
        if !matches!(url.scheme(), "http" | "https") {
            return value.into();
        }
        let fragment = url.fragment().map(str::to_owned);
        url.set_fragment(None);
        if let Some(target) = mapping.get(url.as_str()) {
            let mut value = path_url(target);
            if let Some(fragment) = fragment {
                value.push('#');
                value.push_str(&fragment);
            }
            value
        } else {
            if !navigation {
                online.borrow_mut().insert(url.as_str().to_owned());
            }
            if let Some(fragment) = fragment {
                url.set_fragment(Some(&fragment));
            }
            url.into()
        }
    };
    let output = if item.kind == ItemKind::Stylesheet {
        document::css(&input, &mut |v, _| resolve(v, false), 0)
    } else {
        document::html(&input, &base, &mut |v, navigation, _| {
            resolve(v, navigation)
        })?
    };
    Ok((output, online.into_inner()))
}
