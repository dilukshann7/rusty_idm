//! Shared classification. A manual override never changes a destination.
use std::path::Path;
pub const BUILTIN: [&str; 5] = ["Compressed", "Documents", "Music", "Programs", "Video"];
pub fn infer(path: &Path) -> Option<&'static str> {
    let extension = path.extension()?.to_str()?.to_ascii_lowercase();
    Some(match extension.as_str() {
        "zip" | "7z" | "rar" | "tar" | "gz" | "bz2" | "xz" | "tgz" | "zst" | "cab" | "iso" => {
            "Compressed"
        }
        "pdf" | "txt" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "odt" | "ods"
        | "odp" | "rtf" | "csv" | "epub" | "md" | "html" | "htm" | "xhtml" => "Documents",
        "mp3" | "wav" | "flac" | "m4a" | "aac" | "ogg" | "opus" | "wma" | "aiff" => "Music",
        "exe" | "msi" | "msix" | "msixbundle" | "appx" | "appxbundle" | "com" | "bat" | "cmd" => {
            "Programs"
        }
        "mp4" | "mkv" | "webm" | "avi" | "mov" | "wmv" | "m4v" | "mpeg" | "mpg" | "ts" | "flv"
        | "3gp" => "Video",
        _ => return None,
    })
}
pub fn effective<'a>(path: &Path, manual: Option<&'a str>) -> Option<&'a str> {
    manual
        .filter(|s| !s.eq_ignore_ascii_case("Automatic"))
        .or_else(|| infer(path))
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CategoryRule {
    pub name: String,
    pub extensions: Vec<String>,
    pub folder: Option<std::path::PathBuf>,
    pub remember: bool,
}
pub fn rules() -> Vec<CategoryRule> {
    let groups = [
        ("General", ""),
        ("Compressed", "zip 7z rar tar gz bz2 xz tgz zst cab iso"),
        (
            "Documents",
            "pdf txt doc docx xls xlsx ppt pptx odt ods odp rtf csv epub md html htm xhtml",
        ),
        ("Music", "mp3 wav flac m4a aac ogg opus wma aiff"),
        (
            "Programs",
            "exe msi msix msixbundle appx appxbundle com bat cmd apk",
        ),
        ("Video", "mp4 mkv webm avi mov wmv m4v mpeg mpg ts flv 3gp"),
    ];
    groups
        .into_iter()
        .map(|(name, ext)| CategoryRule {
            name: name.into(),
            extensions: ext.split_whitespace().map(str::to_owned).collect(),
            folder: None,
            remember: true,
        })
        .collect()
}
pub fn validate_rules(rules: &[CategoryRule]) -> Result<(), String> {
    if rules.len() < 6 || rules.len() > 128 {
        return Err(
            "Keep General and the five built-in categories; maximum 128 categories.".into(),
        );
    }
    let mut names = std::collections::BTreeSet::new();
    let mut assigned = std::collections::BTreeMap::new();
    for rule in rules {
        let name = rule.name.trim();
        if name.is_empty()
            || name.chars().count() > 80
            || name.chars().any(char::is_control)
            || name.eq_ignore_ascii_case("Automatic")
            || !names.insert(name.to_lowercase())
        {
            return Err(
                "Category names must be unique, nonempty and at most 80 characters.".into(),
            );
        }
        for ext in &rule.extensions {
            let normalized = rusty_browser_bridge::policy::extensions(ext)?;
            if normalized.len() != 1 || &normalized[0] != ext {
                return Err("Normalize category extensions before saving.".into());
            }
            if let Some(prior) = assigned.insert(ext, name) {
                return Err(format!("{ext} already belongs to {prior}. Remove it there before assigning it to {name}."));
            }
        }
    }
    for required in std::iter::once("General").chain(BUILTIN) {
        if !rules.iter().any(|r| r.name == required) {
            return Err(format!("The {required} category is permanent."));
        }
    }
    if rules
        .iter()
        .any(|r| r.name == "General" && !r.extensions.is_empty())
    {
        return Err("General contains otherwise unmatched types and has no extension list.".into());
    }
    Ok(())
}
pub fn classify<'a>(path: &Path, rules: &'a [CategoryRule]) -> Option<&'a CategoryRule> {
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
    rules
        .iter()
        .find(|r| r.extensions.iter().any(|e| e.eq_ignore_ascii_case(ext)))
        .or_else(|| rules.iter().find(|r| r.name == "General"))
}
