//! Explicit, background-only preference application with ownership-checked compensation.
use crate::{
    configuration_system as system,
    settings::{self, Settings},
};
use rusty_engine::{CategoryRenameUndo, Command, EngineClient};
use std::{
    path::{Path, PathBuf},
    sync::mpsc,
};
struct Files {
    before: Vec<(PathBuf, Option<Vec<u8>>)>,
    after: Vec<(PathBuf, Option<Vec<u8>>)>,
    key: String,
    before_key: Option<String>,
    after_valid: bool,
    after_key: Option<String>,
}
fn read(path: &Path) -> Result<Option<Vec<u8>>, String> {
    match std::fs::metadata(path) {
        Ok(m) if m.len() > 64 * 1024 * 1024 => {
            Err("An owned integration file is unexpectedly large.".into())
        }
        Ok(_) => std::fs::read(path)
            .map(Some)
            .map_err(|_| "Cannot read owned integration files.".into()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(_) => Err("Cannot inspect owned integration files.".into()),
    }
}
fn restore_file(path: &Path, value: &Option<Vec<u8>>) -> Result<(), String> {
    if let Some(bytes) = value {
        let temp = path.with_extension(format!("restore-{}.tmp", std::process::id()));
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
            .map_err(|_| "Cannot reserve compensation file.".to_owned())?;
        use std::io::Write;
        let result = (|| {
            file.write_all(bytes)?;
            file.sync_all()?;
            drop(file);
            settings::atomic_replace(&temp, path)
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(&temp);
        }
        result.map_err(|_| "Cannot restore an owned integration file.".into())
    } else {
        match std::fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(_) => Err("Cannot remove a newly created integration file.".into()),
        }
    }
}
impl Files {
    fn capture(browser: rusty_browser_bridge::policy::BrowserKind) -> Result<Self, String> {
        let root = system::browser_root(browser)?;
        let mut paths = vec![
            root.join("native-host.json"),
            root.join("host-config.json"),
            root.join("registration-owner.json"),
        ];
        if browser != rusty_browser_bridge::policy::BrowserKind::Chrome {
            paths.push(root.join("rusty-native-host.exe"));
        }
        let before = paths
            .into_iter()
            .map(|p| Ok((p.clone(), read(&p)?)))
            .collect::<Result<_, String>>()?;
        let key = system::browser_key(browser);
        let before_key = system::read_value(&key, "")?;
        Ok(Self {
            before,
            after: vec![],
            key,
            before_key,
            after_key: None,
            after_valid: false,
        })
    }
    fn accepted(&mut self) -> Result<(), String> {
        self.after = self
            .before
            .iter()
            .map(|(p, _)| Ok((p.clone(), read(p)?)))
            .collect::<Result<_, String>>()?;
        self.after_key = system::read_value(&self.key, "")?;
        self.after_valid = true;
        Ok(())
    }
    fn rollback(&self) -> Result<(), String> {
        if !self.after_valid {
            return Err("Browser setup succeeded but its resulting files could not be verified; the registration is preserved for explicit repair.".into());
        }
        if system::read_value(&self.key, "")? != self.after_key {
            return Err(
                "A browser registration changed externally; it was preserved for repair.".into(),
            );
        }
        for (p, expected) in &self.after {
            if &read(p)? != expected {
                return Err(
                    "An integration file changed externally; it was preserved for repair.".into(),
                );
            }
        }
        for (p, before) in &self.before {
            restore_file(p, before)?;
        }
        system::write_value(&self.key, "", self.before_key.as_deref())
    }
}
fn register(registration: &crate::preferences::BrowserRegistration) -> Result<(), String> {
    let gui = std::env::current_exe().map_err(|_| "Cannot locate the running application.")?;
    let bin = gui
        .parent()
        .ok_or("Cannot locate the application directory.")?;
    let host = bin.join("rusty-native-host.exe");
    let installed = bin.parent().map(|p| p.join("scripts/browser-host.ps1"));
    let script = installed
        .filter(|p| p.is_file())
        .or_else(|| {
            let p =
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scripts/browser-host.ps1");
            p.is_file().then_some(p)
        })
        .ok_or("Browser registration helper is missing from this application package.")?;
    if !host.is_file() {
        return Err("The native host executable is missing beside Rusty. Build/package both applications before connecting browsers.".into());
    }
    let shell = PathBuf::from(std::env::var_os("WINDIR").ok_or("Windows directory unavailable.")?)
        .join("System32/WindowsPowerShell/v1.0/powershell.exe");
    use std::os::windows::process::CommandExt;
    let result = std::process::Command::new(shell)
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
        ])
        .arg(script)
        .args([
            "-Browser",
            registration.browser.key(),
            "-ExtensionId",
            &registration.extension_id,
            "-HostExe",
        ])
        .arg(host)
        .arg("-GuiExe")
        .arg(gui)
        .creation_flags(0x08000000)
        .output()
        .map_err(|_| "Cannot run the owned browser setup helper.".to_owned())?;
    if !result.status.success() {
        return Err(format!(
            "Browser registration failed: {}",
            String::from_utf8_lossy(&result.stderr)
                .chars()
                .take(4000)
                .collect::<String>()
        ));
    }
    Ok(())
}
fn rename(
    engine: &EngineClient,
    renames: Vec<(String, String)>,
) -> Result<CategoryRenameUndo, String> {
    if renames.is_empty() {
        return Ok(CategoryRenameUndo::default());
    }
    let (tx, rx) = tokio::sync::oneshot::channel();
    engine
        .try_send(Command::RenameCategories(renames, tx))
        .map_err(|e| e.to_string())?;
    rx.blocking_recv()
        .map_err(|_| "The engine closed before category changes were acknowledged.".to_owned())?
}
fn undo(engine: &EngineClient, receipt: CategoryRenameUndo) -> Result<(), String> {
    if receipt.jobs.is_empty() && receipt.entries.is_empty() {
        return Ok(());
    }
    let (tx, rx) = tokio::sync::oneshot::channel();
    engine
        .try_send(Command::RestoreCategories(receipt, tx))
        .map_err(|e| e.to_string())?;
    rx.blocking_recv().map_err(|_| {
        "The engine closed before category compensation was acknowledged.".to_owned()
    })?
}
pub fn begin(
    mut next: Settings,
    previous: Settings,
    engine: EngineClient,
    owner: windows::Win32::Foundation::HWND,
) -> Result<mpsc::Receiver<Result<Settings, String>>, String> {
    let (tx, rx) = mpsc::sync_channel(1);
    let owner = owner.0 as usize;
    std::thread::Builder::new()
        .name("rusty-apply-settings".into())
        .spawn(move || {
            let result = apply(&mut next, &previous, &engine, owner).map(|_| next);
            let _ = tx.send(result);
        })
        .map_err(|_| "Cannot start the preference save worker.".to_owned())?;
    Ok(rx)
}
fn apply(
    next: &mut Settings,
    previous: &Settings,
    engine: &EngineClient,
    owner: usize,
) -> Result<(), String> {
    settings::validate(next).map_err(|e| e.to_string())?;
    let mut browser_changes = vec![];
    let startup_before = system::read_value(system::RUN_KEY, "RustyDownloadManager")?;
    let startup = system::startup_command()?;
    let startup_after = match next.configuration.startup {
        Some(true) => {
            if startup_before.as_ref().is_some_and(|v| v != &startup) {
                return Err(
                    "Another executable owns the Rusty startup entry. It was preserved.".into(),
                );
            }
            Some(startup.clone())
        }
        Some(false) if startup_before.as_ref() == Some(&startup) => None,
        _ => startup_before.clone(),
    };
    let mut category_undo = CategoryRenameUndo::default();
    let mut changed_startup = false;
    let mut changed_clipboard = false;
    let result = (|| {
        for registration in &next.configuration.registrations {
            let mut snapshot = Files::capture(registration.browser)?;
            register(registration)?;
            let accepted = snapshot.accepted();
            browser_changes.push(snapshot);
            accepted?;
        }
        if startup_before != startup_after {
            if system::read_value(system::RUN_KEY, "RustyDownloadManager")? != startup_before {
                return Err("Startup registration changed externally. No startup preference was overwritten.".into());
            }
            system::write_value(
                system::RUN_KEY,
                "RustyDownloadManager",
                startup_after.as_deref(),
            )?;
            changed_startup = true;
        }
        if next.configuration.clipboard != previous.configuration.clipboard {
            clipboard_listener(owner, next.configuration.clipboard)?;
            changed_clipboard = true;
        }
        category_undo = rename(engine, next.configuration.renamed_categories.clone())?;
        for (from, to) in &next.configuration.renamed_categories {
            if let Some(path) = next.category_destinations.remove(&from.to_lowercase()) {
                next.category_destinations.insert(to.to_lowercase(), path);
            }
        }
        for rule in &next.configuration.categories {
            if let Some(folder) = &rule.folder {
                next.category_destinations
                    .insert(rule.name.to_lowercase(), folder.clone());
            }
        }
        next.schema_version = 2;
        next.configuration.revision = previous.configuration.revision.saturating_add(1);
        next.configuration.capture.revision = next.configuration.revision;
        settings::save(next).map_err(|e| e.to_string())?;
        next.configuration.registrations.clear();
        next.configuration.renamed_categories.clear();
        Ok(())
    })();
    if let Err(failure) = result {
        let mut repair = vec![];
        if changed_clipboard {
            if let Err(error) = clipboard_listener(owner, previous.configuration.clipboard) {
                repair.push(error);
            }
        }
        if let Err(e) = undo(engine, category_undo) {
            repair.push(e);
        }
        if changed_startup {
            match system::read_value(system::RUN_KEY, "RustyDownloadManager") {
                Ok(value) if value == startup_after => {
                    if let Err(e) = system::write_value(
                        system::RUN_KEY,
                        "RustyDownloadManager",
                        startup_before.as_deref(),
                    ) {
                        repair.push(e);
                    }
                }
                _ => repair.push("Startup changed externally; it was preserved.".into()),
            }
        }
        for snapshot in browser_changes.into_iter().rev() {
            if let Err(e) = snapshot.rollback() {
                repair.push(e);
            }
        }
        if repair.is_empty() {
            return Err(failure);
        }
        return Err(format!(
            "{failure}\n\nSome external settings need repair: {}",
            repair.join("; ")
        ));
    }
    Ok(())
}

fn clipboard_listener(owner: usize, enabled: bool) -> Result<(), String> {
    use windows::Win32::{
        Foundation::HWND,
        System::DataExchange::{AddClipboardFormatListener, RemoveClipboardFormatListener},
    };
    let hwnd = HWND(owner as *mut _);
    unsafe {
        if enabled {
            AddClipboardFormatListener(hwnd)
        } else {
            RemoveClipboardFormatListener(hwnd)
        }
    }
    .map_err(|_| {
        "Cannot update the clipboard subscription; the previous preference was retained.".into()
    })
}
