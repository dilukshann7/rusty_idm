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
    use rusty_native_host::{config::HostConfig, host::run_host};
    let root = std::env::var_os("LOCALAPPDATA")
        .ok_or_else(|| std::io::Error::other("Missing local application directory"))?;
    let config = HostConfig::load(
        &std::path::PathBuf::from(root).join("RustyIDM/browser/host-config.json"),
    )?;
    if std::fs::canonicalize(std::env::current_exe()?)? != std::fs::canonicalize(&config.host_exe)?
    {
        return Err(std::io::Error::other("Unexpected native host executable"));
    }
    let origin = std::env::args()
        .nth(1)
        .ok_or_else(|| std::io::Error::other("Missing native caller"))?;
    run_host(
        &mut std::io::stdin(),
        &mut std::io::stdout(),
        &config,
        &origin,
    )
}
