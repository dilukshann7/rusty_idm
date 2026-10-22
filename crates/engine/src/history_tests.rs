use super::*;
use crate::storage::{Recovery, Store, StoredEntry};
use std::time::{Duration, SystemTime};

fn entry(id: u64, destination: &std::path::Path) -> StoredEntry {
    serde_json::from_value(serde_json::json!({
        "id": id, "url": "https://example.com/file?private=value",
        "destination": destination, "category": "Documents", "description": "Saved description",
        "authenticated": false, "completed": false, "checkpoint": null, "journal": null
    }))
    .unwrap()
}

fn recovery() -> (tempfile::TempDir, Recovery) {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(false);
    store
        .update(|state| {
            state.next_entry = 2;
            state
                .entries
                .insert(crate::EntryId(1), entry(1, &dir.path().join("file.bin")));
            Ok(())
        })
        .unwrap();
    (
        dir,
        Recovery {
            store,
            entry: crate::EntryId(1),
        },
    )
}

#[test]
fn completed_history_keeps_actual_bytes_dates_and_private_metadata() {
    let (_dir, recovery) = recovery();
    let created = SystemTime::UNIX_EPOCH + Duration::from_secs(100);
    let mut history = HistoryMetadata {
        created_at: Some(created),
        last_try: Some(created),
        ..Default::default()
    };
    history.outcome = Some(HistoryOutcome::Completed);
    history.finished_at = Some(created + Duration::from_secs(10));
    history.received = 123;
    history.total = Some(123);
    recovery.save_history(history.clone()).unwrap();
    let saved = recovery.store.snapshot().entries[&recovery.entry].clone();
    assert!(saved.completed);
    assert_eq!(saved.completed_bytes, Some(123));
    assert_eq!(saved.history, history);
    assert_eq!(saved.category.as_deref(), Some("Documents"));
    assert!(saved.url.contains("private=value"));
    assert!(saved.checkpoint.is_none());
}

#[test]
fn failure_history_is_sanitized_and_keeps_zero_unknown_size() {
    let (_dir, recovery) = recovery();
    let error = crate::JobError::new(
        crate::ErrorKind::Network,
        "secret cookie=value https://example.com/file?token=secret Authorization: private",
    );
    let failure = HistoryFailure::from_error(&error);
    let history = HistoryMetadata {
        outcome: Some(HistoryOutcome::Failed),
        error: Some(failure),
        ..Default::default()
    };
    recovery.save_history(history).unwrap();
    let saved = recovery.store.snapshot().entries[&recovery.entry]
        .history
        .clone();
    assert_eq!(saved.received, 0);
    assert_eq!(saved.total, None);
    let encoded = serde_json::to_string(&saved).unwrap();
    assert!(!encoded.contains("secret"));
    assert!(!encoded.contains("Authorization"));
    assert_eq!(
        saved.error.unwrap().to_error().kind,
        crate::ErrorKind::Network
    );
}

#[test]
fn checkpoint_cleanup_preserves_history_until_deletion_is_durable() {
    let (_dir, recovery) = recovery();
    let created = SystemTime::now();
    recovery
        .save_history(HistoryMetadata {
            created_at: Some(created),
            outcome: Some(HistoryOutcome::Failed),
            finished_at: Some(created),
            error: Some(HistoryFailure::from_error(&crate::JobError::new(
                crate::ErrorKind::Http,
                "HTTP 403",
            ))),
            ..Default::default()
        })
        .unwrap();
    recovery.clear().unwrap();
    let saved = recovery.store.snapshot().entries[&recovery.entry]
        .history
        .clone();
    assert_eq!(saved.created_at, Some(created));
    assert_eq!(saved.outcome, Some(HistoryOutcome::Failed));
    assert_eq!(saved.finished_at, Some(created));
    assert!(saved.error.is_some());
}

#[test]
fn history_capacity_allows_1001_and_rejects_10001_without_pruning() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(false);
    store
        .update(|state| {
            state.next_entry = 10_001;
            for id in 1..=10_000 {
                state.entries.insert(
                    crate::EntryId(id),
                    entry(id, &dir.path().join(format!("{id}.bin"))),
                );
            }
            Ok(())
        })
        .unwrap();
    assert!(store.snapshot().entries.contains_key(&crate::EntryId(1001)));
    let error = store
        .update(|state| {
            state.next_entry += 1;
            state.entries.insert(
                crate::EntryId(10_001),
                entry(10_001, &dir.path().join("extra.bin")),
            );
            Ok(())
        })
        .unwrap_err();
    assert!(error.contains("history is full"));
    assert_eq!(store.snapshot().entries.len(), 10_000);
    store
        .update(|state| {
            state.entries.remove(&crate::EntryId(1));
            Ok(())
        })
        .unwrap();
    assert_eq!(store.snapshot().entries.len(), 9_999);
}

#[test]
fn legacy_metadata_defaults_to_unknown_dates() {
    let dir = tempfile::tempdir().unwrap();
    let mut saved = entry(1, &dir.path().join("legacy.bin"));
    saved.completed = true;
    saved.completed_bytes = Some(42);
    assert_eq!(saved.history.created_at, None);
    assert_eq!(saved.history.last_try, None);
    assert_eq!(saved.history.finished_at, None);
    assert!(saved.completed);
}
