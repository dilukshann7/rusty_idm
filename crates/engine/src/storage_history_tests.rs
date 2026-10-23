use super::*;
use crate::history::{HistoryMetadata, HistoryOutcome};

fn state(dir: &std::path::Path) -> State {
    let mut state = State::default();
    state.next_entry = 2;
    state.entries.insert(
        EntryId(1),
        serde_json::from_value(serde_json::json!({
            "id": 1, "url": "https://example.com/file", "destination": dir.join("file.bin"),
            "category": "Documents", "description": "Legacy", "authenticated": false,
            "completed": true, "completed_bytes": 42, "checkpoint": null, "journal": null
        }))
        .unwrap(),
    );
    state
        .queues
        .get_mut(&QueueId::MAIN)
        .unwrap()
        .entries
        .push(EntryId(1));
    state
}

#[test]
fn schemas_1_through_6_migrate_without_invented_dates_or_changed_ids() {
    let dir = tempfile::tempdir().unwrap();
    for version in 1..=6 {
        let mut value = serde_json::to_value(state(dir.path())).unwrap();
        value["version"] = version.into();
        value["entries"]["1"]
            .as_object_mut()
            .unwrap()
            .remove("history");
        let decoded = decode_state(&serde_json::to_vec(&value).unwrap()).unwrap();
        assert_eq!(decoded.version, 8);
        let entry = &decoded.entries[&EntryId(1)];
        assert!(entry.completed);
        assert_eq!(entry.completed_bytes, Some(42));
        assert_eq!(entry.history, HistoryMetadata::default());
        assert_eq!(decoded.queues[&QueueId::MAIN].entries, vec![EntryId(1)]);
        assert_eq!(decoded.next_entry, 2);
    }
    assert!(decode_state(b"corrupt data").is_err());
    let mut future = state(dir.path());
    future.version = 9;
    assert!(decode_state(&serde_json::to_vec(&future).unwrap()).is_err());
}

#[test]
fn size_limit_keeps_previous_state_and_deletion_frees_capacity() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(false);
    store
        .update(|saved| {
            *saved = state(dir.path());
            Ok(())
        })
        .unwrap();
    let error = store
        .update(|saved| {
            saved.entries.get_mut(&EntryId(1)).unwrap().description = "x".repeat(MAX_STORAGE_BYTES);
            Ok(())
        })
        .unwrap_err();
    assert!(error.contains("history is full"));
    assert_eq!(store.snapshot().entries[&EntryId(1)].description, "Legacy");
    store
        .update(|saved| {
            saved.entries.remove(&EntryId(1));
            saved
                .queues
                .get_mut(&QueueId::MAIN)
                .unwrap()
                .entries
                .clear();
            Ok(())
        })
        .unwrap();
    assert!(store.snapshot().entries.is_empty());
}

#[test]
fn failed_terminal_save_remains_pending_without_reclassifying_success() {
    let dir = tempfile::tempdir().unwrap();
    let mut saved = state(dir.path());
    saved.entries.get_mut(&EntryId(1)).unwrap().completed = false;
    let mut store = Store {
        path: None,
        state: Mutex::new(saved),
        pending_history: Mutex::new(BTreeMap::new()),
        blocked: Some("Injected write failure".into()),
        fail_writes: std::sync::atomic::AtomicBool::new(false),
    };
    let history = HistoryMetadata {
        outcome: Some(HistoryOutcome::Completed),
        received: 777,
        total: Some(777),
        finished_at: Some(std::time::SystemTime::now()),
        ..Default::default()
    };
    assert!(store.save_history(EntryId(1), history.clone()).is_err());
    assert!(store.has_pending_history());
    assert_eq!(store.snapshot().entries[&EntryId(1)].history, history);
    assert!(store.snapshot().entries[&EntryId(1)].completed);
    assert!(!store.state.lock().unwrap().entries[&EntryId(1)].completed);
    store.blocked = None;
    store.flush_history().unwrap();
    assert!(!store.has_pending_history());
    assert_eq!(
        store.reloaded_for_test().snapshot().entries[&EntryId(1)].completed_bytes,
        Some(777)
    );
}

#[test]
fn failed_deletion_preserves_record_queue_membership_and_file() {
    let dir = tempfile::tempdir().unwrap();
    let destination = dir.path().join("file.bin");
    std::fs::write(&destination, b"keep me").unwrap();
    let store = Store {
        path: None,
        state: Mutex::new(state(dir.path())),
        pending_history: Mutex::new(BTreeMap::new()),
        blocked: Some("Injected write failure".into()),
        fail_writes: std::sync::atomic::AtomicBool::new(false),
    };
    assert!(store
        .update(|state| {
            state.entries.remove(&EntryId(1));
            state
                .queues
                .get_mut(&QueueId::MAIN)
                .unwrap()
                .entries
                .clear();
            Ok(())
        })
        .is_err());
    assert!(store.snapshot().entries.contains_key(&EntryId(1)));
    assert_eq!(
        store.snapshot().queues[&QueueId::MAIN].entries,
        vec![EntryId(1)]
    );
    assert_eq!(std::fs::read(destination).unwrap(), b"keep me");
}

#[cfg(windows)]
#[test]
fn encrypted_roundtrip_and_corrupt_storage_preserve_isolated_files() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("history.dpapi");
    let store = Store::open_path(Some(path.clone()));
    store
        .update(|saved| {
            *saved = state(dir.path());
            Ok(())
        })
        .unwrap();
    assert!(!std::fs::read(&path)
        .unwrap()
        .windows(b"example.com".len())
        .any(|bytes| bytes == b"example.com"));
    let reopened = Store::open_path(Some(path.clone()));
    assert!(reopened.problem().is_none());
    assert_eq!(
        reopened.snapshot().entries[&EntryId(1)].completed_bytes,
        Some(42)
    );
    std::fs::write(&path, b"corrupt original").unwrap();
    let blocked = Store::open_path(Some(path.clone()));
    assert!(blocked.problem().is_some());
    assert!(blocked.update(|_| Ok(())).is_err());
    assert_eq!(std::fs::read(path).unwrap(), b"corrupt original");
}
