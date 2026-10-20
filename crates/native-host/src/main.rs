fn main() {
    #[cfg(windows)]
    if run().is_err() {
        eprintln!("Rusty browser host is unavailable. Check per-user setup.");
        std::process::exit(1);
    }
    #[cfg(not(windows))]
    {
        eprintln!("Rusty browser host requires Windows.");
        std::process::exit(1);
    }
}
#[cfg(windows)]
fn run() -> std::io::Result<()> {
    let _installer_guard = rusty_browser_bridge::windows::OwnedHandle(
        unsafe {
            windows::Win32::System::Threading::CreateMutexW(
                None,
                false,
                windows::core::w!("Local\\RustyDownloadManager.NativeHost.Running"),
            )
        }
        .map_err(std::io::Error::from)?,
    );
    use rusty_native_host::{config::HostConfig, host::run_host};
    let root = std::env::var_os("LOCALAPPDATA")
        .ok_or_else(|| std::io::Error::other("Missing local application directory"))?;
    let local = std::env::current_exe()?
        .parent()
        .ok_or_else(|| std::io::Error::other("Missing native host parent"))?
        .join("host-config.json");
    let config = HostConfig::load(&if local.is_file() {
        local
    } else {
        std::path::PathBuf::from(root).join("RustyIDM/browser/host-config.json")
    })?;
    if std::fs::canonicalize(std::env::current_exe()?)? != std::fs::canonicalize(&config.host_exe)?
    {
        return Err(std::io::Error::other("Unexpected native host executable"));
    }
    let origin = std::env::args()
        .nth(
            if config.browser == rusty_browser_bridge::policy::BrowserKind::Firefox {
                2
            } else {
                1
            },
        )
        .ok_or_else(|| std::io::Error::other("Missing native caller"))?;
    run_host(
        &mut std::io::stdin(),
        &mut std::io::stdout(),
        &config,
        &origin,
    )
}
