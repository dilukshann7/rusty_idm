#![cfg_attr(windows, windows_subsystem = "windows")]

fn main() {
    #[cfg(windows)]
    if let Err(error) = rusty_win32::run() {
        rusty_win32::platform::message(None, "Rusty Download Manager", &error.to_string(), false);
    }
    #[cfg(not(windows))]
    eprintln!("Rusty Download Manager uses native Windows controls. Run it on Windows 10/11.");
}
