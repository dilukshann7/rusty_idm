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

#[cfg(windows)]
#[test]
fn status_and_abort_without_a_gui_do_not_launch_the_configured_executable() {
    use rusty_browser_bridge::{framing::*, protocol::*};
    let config = HostConfig {
        extension_id: "a".repeat(32),
        gui_exe: "C:/missing-Rusty-fixture.exe".into(),
        host_exe: "C:/missing-host-fixture.exe".into(),
    };
    let target = RequestId::parse("12345678-1234-4234-8234-123456789abc").unwrap();
    let mut bytes = Vec::new();
    for op in [
        Operation::Status {
            capture_id: target.clone(),
        },
        Operation::Abort {
            capture_id: target.clone(),
        },
    ] {
        write_frame(&mut bytes, &op.encode(target.clone()).unwrap()).unwrap();
    }
    let mut output = Vec::new();
    run_host(
        &mut std::io::Cursor::new(bytes),
        &mut output,
        &config,
        &format!("chrome-extension://{}/", config.extension_id),
    )
    .unwrap();
    let mut output = std::io::Cursor::new(output);
    let mut count = 0;
    while let Some(frame) = read_frame(&mut output).unwrap() {
        let reply: Reply = serde_json::from_slice(&frame).unwrap();
        assert_eq!(reply.result, ReplyStatus::Unknown);
        count += 1;
    }
    assert_eq!(count, 2);
}

#[cfg(windows)]
#[test]
fn spoofed_origin_produces_no_stdout_and_never_launches_gui() {
    let config = HostConfig {
        extension_id: "a".repeat(32),
        gui_exe: "C:/missing-Rusty-fixture.exe".into(),
        host_exe: "C:/missing-host-fixture.exe".into(),
    };
    let mut output = Vec::new();
    assert!(run_host(
        &mut std::io::Cursor::new(Vec::<u8>::new()),
        &mut output,
        &config,
        "chrome-extension://bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb/"
    )
    .is_err());
    assert!(output.is_empty());
}
