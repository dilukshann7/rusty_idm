use super::*;

#[test]
fn rejects_corrupt_and_unknown_schema_without_overwriting() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    for text in [
        "{broken",
        r#"{"schema_version":99,"default_destination":"C:/Downloads"}"#,
    ] {
        std::fs::write(&path, text).unwrap();
        assert!(load_from(&path).is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), text);
    }
}
#[test]
fn unicode_settings_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    let folder = dir.path().join("தமிழ் 日本語");
    std::fs::create_dir(&folder).unwrap();
    let settings = Settings {
        schema_version: 1,
        category_destinations: Default::default(),
        duplicate_choice: None,
        show_completion_dialog: true,
        default_destination: folder.clone(),
    };
    save_to(&path, &settings).unwrap();
    assert_eq!(load_from(&path).unwrap().default_destination, folder);
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 2);
}
#[test]
fn failed_save_preserves_prior_settings() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    let settings = Settings {
        schema_version: 1,
        category_destinations: Default::default(),
        duplicate_choice: None,
        show_completion_dialog: true,
        default_destination: dir.path().into(),
    };
    save_to(&path, &settings).unwrap();
    let old = std::fs::read(&path).unwrap();
    let invalid = Settings {
        schema_version: 99,
        ..settings
    };
    assert!(save_to(&path, &invalid).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), old);
    let blocked_parent = dir.path().join("blocked");
    std::fs::write(&blocked_parent, b"file").unwrap();
    let valid = Settings {
        schema_version: 1,
        category_destinations: Default::default(),
        duplicate_choice: None,
        show_completion_dialog: true,
        default_destination: dir.path().into(),
    };
    assert!(save_to(&blocked_parent.join("settings.json"), &valid).is_err());
}

#[cfg(windows)]
#[test]
fn failed_atomic_replace_preserves_prior_file_and_removes_temporary() {
    use std::os::windows::fs::OpenOptionsExt;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    let old = Settings {
        schema_version: 1,
        category_destinations: Default::default(),
        duplicate_choice: None,
        show_completion_dialog: true,
        default_destination: dir.path().into(),
    };
    save_to(&path, &old).unwrap();
    let original = std::fs::read(&path).unwrap();
    let folder = dir.path().join("next");
    std::fs::create_dir(&folder).unwrap();
    let new = Settings {
        schema_version: 1,
        category_destinations: Default::default(),
        duplicate_choice: None,
        show_completion_dialog: true,
        default_destination: folder,
    };
    let lock = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(3)
        .open(&path)
        .unwrap();
    assert!(save_to(&path, &new).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), original);
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 2);
    drop(lock);
}

#[test]
fn legacy_settings_still_load_and_category_paths_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    let legacy = serde_json::json!({ "schema_version": 1, "default_destination": dir.path() });
    std::fs::write(&path, serde_json::to_vec(&legacy).unwrap()).unwrap();
    let mut settings = load_from(&path).unwrap();
    assert!(settings.category_destinations.is_empty());
    assert!(settings.duplicate_choice.is_none());
    settings
        .category_destinations
        .insert("documents".into(), dir.path().to_owned());
    save_to(&path, &settings).unwrap();
    assert_eq!(
        load_from(&path)
            .unwrap()
            .category_destinations
            .get("documents"),
        Some(&dir.path().to_owned())
    );
}

#[test]
fn duplicate_preference_roundtrips_without_storing_a_link() {
    use crate::duplicates::DuplicateChoice;

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    let mut settings = Settings {
        schema_version: 1,
        category_destinations: Default::default(),
        duplicate_choice: None,
        show_completion_dialog: true,
        default_destination: dir.path().into(),
    };
    for (choice, expected) in [
        (DuplicateChoice::Numbered, "numbered"),
        (DuplicateChoice::Overwrite, "overwrite"),
        (DuplicateChoice::Existing, "existing"),
    ] {
        settings.duplicate_choice = Some(choice);
        save_to(&path, &settings).unwrap();
        let loaded = load_from(&path).unwrap();
        assert_eq!(
            serde_json::to_value(loaded.duplicate_choice).unwrap(),
            serde_json::json!(expected)
        );
        let saved: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(saved.as_object().unwrap().len(), 5);
    }
    settings.duplicate_choice = None;
    save_to(&path, &settings).unwrap();
    assert!(load_from(&path).unwrap().duplicate_choice.is_none());
}

#[test]
fn unknown_duplicate_preference_preserves_saved_settings() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    let invalid = serde_json::json!({
        "schema_version": 1,
        "default_destination": dir.path(),
        "duplicate_choice": "unknown_action",
    });
    let data = serde_json::to_vec(&invalid).unwrap();
    std::fs::write(&path, &data).unwrap();
    assert!(load_from(&path).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), data);
}
