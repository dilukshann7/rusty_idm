//! Read-only discovery and owned runtime configuration helpers.
use rusty_browser_bridge::policy::BrowserKind;
use std::path::PathBuf;
use windows::{
    core::PCWSTR,
    Win32::{Foundation::ERROR_FILE_NOT_FOUND, System::Registry::*},
};
pub const RUN_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
pub fn read_value(key: &str, name: &str) -> Result<Option<String>, String> {
    let key = crate::platform::wide(key);
    let name = crate::platform::wide(name);
    let mut size = 0u32;
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            PCWSTR(key.as_ptr()),
            PCWSTR(name.as_ptr()),
            RRF_RT_REG_SZ,
            None,
            None,
            Some(&mut size),
        )
    };
    if status == ERROR_FILE_NOT_FOUND {
        return Ok(None);
    }
    if status.0 != 0 || size > 65536 {
        return Err("Cannot read the owned Windows setting.".into());
    }
    let mut value = vec![0u16; (size as usize + 1) / 2];
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            PCWSTR(key.as_ptr()),
            PCWSTR(name.as_ptr()),
            RRF_RT_REG_SZ,
            None,
            Some(value.as_mut_ptr().cast()),
            Some(&mut size),
        )
    };
    if status.0 != 0 {
        return Err("Windows setting changed while reading.".into());
    }
    let length = value.iter().position(|v| *v == 0).unwrap_or(value.len());
    String::from_utf16(&value[..length])
        .map(Some)
        .map_err(|_| "Invalid Windows setting text.".into())
}
pub fn startup_command() -> Result<String, String> {
    Ok(format!(
        "\"{}\" --background",
        std::env::current_exe()
            .map_err(|_| "Cannot locate Rusty executable.")?
            .display()
    ))
}
pub fn startup_enabled() -> Result<bool, String> {
    Ok(
        read_value(RUN_KEY, "RustyDownloadManager")?.as_deref()
            == Some(startup_command()?.as_str()),
    )
}
pub fn browser_root(browser: BrowserKind) -> Result<PathBuf, String> {
    let mut p = PathBuf::from(std::env::var_os("LOCALAPPDATA").ok_or("LocalAppData unavailable.")?)
        .join("RustyIDM/browser");
    if browser != BrowserKind::Chrome {
        p.push(browser.key());
    }
    Ok(p)
}
pub fn browser_key(browser: BrowserKind) -> String {
    let vendor = match browser {
        BrowserKind::Chrome => "Google\\Chrome",
        BrowserKind::Brave => "BraveSoftware\\Brave-Browser",
        BrowserKind::Edge => "Microsoft\\Edge",
        BrowserKind::Firefox => "Mozilla",
    };
    format!(
        "Software\\{vendor}\\NativeMessagingHosts\\dev.rusty_idm.{}",
        browser.key()
    )
}
pub fn registered_id(browser: BrowserKind) -> Option<String> {
    let root = browser_root(browser).ok()?;
    let path = root.join("registration-owner.json");
    let bytes = std::fs::read(path).ok()?;
    if bytes.len() > 65536 {
        return None;
    }
    let owner: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    if owner["app"] != "RustyIDM"
        || owner["schema"] != 1
        || PathBuf::from(owner["manifest"].as_str()?) != root.join("native-host.json")
        || read_value(&browser_key(browser), "")
            .ok()
            .flatten()?
            .as_str()
            != owner["manifest"].as_str()?
    {
        return None;
    }
    owner["extension_id"].as_str().map(str::to_owned)
}
pub fn browser_executable(
    browser: BrowserKind,
    settings: &crate::settings::Settings,
) -> Option<PathBuf> {
    if let Some(path) = settings
        .configuration
        .browser_executables
        .get(&browser)
        .filter(|p| p.is_file())
    {
        return Some(path.clone());
    }
    let tail = match browser {
        BrowserKind::Chrome => "Google/Chrome/Application/chrome.exe",
        BrowserKind::Brave => "BraveSoftware/Brave-Browser/Application/brave.exe",
        BrowserKind::Edge => "Microsoft/Edge/Application/msedge.exe",
        BrowserKind::Firefox => "Mozilla Firefox/firefox.exe",
    };
    ["PROGRAMFILES", "PROGRAMFILES(X86)", "LOCALAPPDATA"]
        .into_iter()
        .filter_map(std::env::var_os)
        .map(|p| PathBuf::from(p).join(tail))
        .find(|p| p.is_file())
}
pub fn executable_kind(path: &std::path::Path) -> Option<BrowserKind> {
    let name = path.file_name()?.to_str()?.to_ascii_lowercase();
    match name.as_str() {
        "chrome.exe" => Some(BrowserKind::Chrome),
        "brave.exe" => Some(BrowserKind::Brave),
        "msedge.exe" => Some(BrowserKind::Edge),
        "firefox.exe" => Some(BrowserKind::Firefox),
        _ => None,
    }
}

/// Used only by the explicit configuration apply/compensation worker.
pub(crate) fn write_value(key: &str, name: &str, value: Option<&str>) -> Result<(), String> {
    let key = crate::platform::wide(key);
    let name = crate::platform::wide(name);
    let mut handle = HKEY::default();
    let status = unsafe {
        if value.is_some() {
            RegCreateKeyExW(
                HKEY_CURRENT_USER,
                PCWSTR(key.as_ptr()),
                None,
                PCWSTR::null(),
                REG_OPTION_NON_VOLATILE,
                KEY_SET_VALUE,
                None,
                &mut handle,
                None,
            )
        } else {
            RegOpenKeyExW(
                HKEY_CURRENT_USER,
                PCWSTR(key.as_ptr()),
                None,
                KEY_SET_VALUE,
                &mut handle,
            )
        }
    };
    if status == ERROR_FILE_NOT_FOUND && value.is_none() {
        return Ok(());
    }
    if status.0 != 0 {
        return Err("Cannot update the owned Windows setting.".into());
    }
    let status = unsafe {
        if let Some(value) = value {
            let value = crate::platform::wide(value);
            let bytes = std::slice::from_raw_parts(value.as_ptr().cast::<u8>(), value.len() * 2);
            RegSetValueExW(handle, PCWSTR(name.as_ptr()), None, REG_SZ, Some(bytes))
        } else {
            RegDeleteValueW(handle, PCWSTR(name.as_ptr()))
        }
    };
    unsafe {
        let _ = RegCloseKey(handle);
    }
    if status.0 == 0 || status == ERROR_FILE_NOT_FOUND {
        Ok(())
    } else {
        Err("Windows rejected the owned setting change.".into())
    }
}
