use rusty_native_host::{config::HostConfig, host::*};
#[test]
fn unicode_spaced_gui_path_is_not_shell_expanded() {
    let config = HostConfig {
        extension_id: "a".repeat(32),
        gui_exe: "C:\\Rusty தமிழ்\\rusty-win32.exe".into(),
        host_exe: "C:\\Rusty தமிழ்\\rusty-native-host.exe".into(),
    };
    let command = gui_command(&config);
    assert_eq!(command.get_program(), config.gui_exe.as_os_str());
    assert_eq!(
        command.get_args().collect::<Vec<_>>(),
        vec!["--browser-integration"]
    );
    assert!(config
        .validate_origin("chrome-extension://bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb/")
        .is_err());
}
#[test]
fn missing_native_configuration_fails_closed() {
    assert!(HostConfig::load(std::path::Path::new("Z:/missing-rusty-config.json")).is_err());
}
