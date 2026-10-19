use rusty_engine::{inspect_destination, output::OutputFile, ErrorKind, ReplacementPolicy};

#[tokio::test]
async fn deny_preserves_existing_data_and_reserves_unique_partials() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("result.bin");
    std::fs::write(&target, b"original").unwrap();
    assert!(OutputFile::prepare(&target, &ReplacementPolicy::Deny)
        .await
        .is_err());
    assert_eq!(std::fs::read(&target).unwrap(), b"original");
    let target = dir.path().join("new.bin");
    let a = OutputFile::prepare(&target, &ReplacementPolicy::Deny)
        .await
        .unwrap();
    let b = OutputFile::prepare(&target, &ReplacementPolicy::Deny)
        .await
        .unwrap();
    assert_ne!(a.partial_path(), b.partial_path());
    let unrelated = dir.path().join("unrelated.part");
    std::fs::write(&unrelated, b"keep").unwrap();
    a.abort().await.unwrap();
    b.abort().await.unwrap();
    assert!(unrelated.exists());
}

#[tokio::test]
async fn late_collision_retains_partial_without_touching_target() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("result.bin");
    let mut output = OutputFile::prepare(&target, &ReplacementPolicy::Deny)
        .await
        .unwrap();
    output.write(b"download").await.unwrap();
    std::fs::write(&target, b"external").unwrap();
    let error = output.finalize().await.unwrap_err();
    assert_eq!(error.kind, ErrorKind::DestinationChanged);
    assert_eq!(
        std::fs::read(error.retained_partial.unwrap()).unwrap(),
        b"download"
    );
    assert_eq!(std::fs::read(target).unwrap(), b"external");
}

#[tokio::test]
async fn duplicate_jobs_cannot_replace_each_other() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("result.bin");
    let mut a = OutputFile::prepare(&target, &ReplacementPolicy::Deny)
        .await
        .unwrap();
    let mut b = OutputFile::prepare(&target, &ReplacementPolicy::Deny)
        .await
        .unwrap();
    a.write(b"first").await.unwrap();
    b.write(b"second").await.unwrap();
    a.finalize().await.unwrap();
    assert!(b.finalize().await.is_err());
    assert_eq!(std::fs::read(target).unwrap(), b"first");
}

#[cfg(not(windows))]
#[tokio::test]
async fn approved_replacement_is_explicitly_disabled() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("result.bin");
    std::fs::write(&target, b"original").unwrap();
    let identity = inspect_destination(&target).unwrap().unwrap();
    assert!(
        OutputFile::prepare(&target, &ReplacementPolicy::Approved(identity))
            .await
            .is_err()
    );
    assert_eq!(std::fs::read(target).unwrap(), b"original");
}

#[cfg(windows)]
#[tokio::test]
async fn approved_replacement_preserves_original_until_completed_and_cleans_backup() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("result.bin");
    std::fs::write(&target, b"original").unwrap();
    let identity = inspect_destination(&target).unwrap().unwrap();
    let mut output = OutputFile::prepare(&target, &ReplacementPolicy::Approved(identity))
        .await
        .unwrap();
    output.write(b"download").await.unwrap();
    assert_eq!(std::fs::read(&target).unwrap(), b"original");
    output.finalize().await.unwrap();
    assert_eq!(std::fs::read(&target).unwrap(), b"download");
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[cfg(windows)]
#[tokio::test]
async fn approved_replacement_abort_drop_and_cancel_preserve_original() {
    use rusty_engine::transfer::TransferControl;
    use tokio_util::sync::CancellationToken;

    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("result.bin");
    std::fs::write(&target, b"original").unwrap();
    for mode in 0..3 {
        let identity = inspect_destination(&target).unwrap().unwrap();
        let mut output = OutputFile::prepare(&target, &ReplacementPolicy::Approved(identity))
            .await
            .unwrap();
        let partial = output.partial_path().to_owned();
        output.write(b"incomplete").await.unwrap();
        match mode {
            0 => output.abort().await.unwrap(),
            1 => drop(output),
            _ => {
                let control = TransferControl::new(CancellationToken::new());
                assert!(control.try_cancel());
                assert!(!output.finalize_controlled(control).await.unwrap());
            }
        }
        assert_eq!(std::fs::read(&target).unwrap(), b"original");
        assert!(!partial.exists());
    }
}

#[cfg(windows)]
#[tokio::test]
async fn approved_replacement_rejects_changed_contents_or_identity() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("result.bin");
    std::fs::write(&target, b"original").unwrap();
    let identity = inspect_destination(&target).unwrap().unwrap();
    std::fs::write(&target, b"changed length").unwrap();
    let error = OutputFile::prepare(&target, &ReplacementPolicy::Approved(identity))
        .await
        .err()
        .expect("stale contents approval must be rejected");
    assert_eq!(error.kind, ErrorKind::DestinationChanged);
    let identity = inspect_destination(&target).unwrap().unwrap();
    std::fs::rename(&target, dir.path().join("old.bin")).unwrap();
    std::fs::write(&target, b"changed length").unwrap();
    assert!(
        OutputFile::prepare(&target, &ReplacementPolicy::Approved(identity))
            .await
            .is_err()
    );
    assert_eq!(std::fs::read(&target).unwrap(), b"changed length");
}

#[cfg(windows)]
#[tokio::test]
async fn approved_original_cannot_be_changed_renamed_or_removed_during_download() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("result.bin");
    std::fs::write(&target, b"original").unwrap();
    let identity = inspect_destination(&target).unwrap().unwrap();
    let output = OutputFile::prepare(&target, &ReplacementPolicy::Approved(identity))
        .await
        .unwrap();
    assert!(std::fs::write(&target, b"external").is_err());
    assert!(std::fs::rename(&target, dir.path().join("moved.bin")).is_err());
    assert!(std::fs::remove_file(&target).is_err());
    output.abort().await.unwrap();
    assert_eq!(std::fs::read(&target).unwrap(), b"original");
}

#[cfg(windows)]
#[tokio::test]
async fn reserved_partial_cannot_be_renamed_or_substituted() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("file.bin");
    let mut output = OutputFile::prepare(&target, &ReplacementPolicy::Deny)
        .await
        .unwrap();
    let partial = output.partial_path().to_owned();
    output.write(b"download").await.unwrap();
    let unrelated = dir.path().join("unrelated.bin");
    std::fs::write(&unrelated, b"keep").unwrap();
    assert!(std::fs::rename(&partial, dir.path().join("moved.part")).is_err());
    assert!(std::fs::rename(&unrelated, &partial).is_err());
    assert!(std::fs::write(&partial, b"substitute").is_err());
    output.finalize().await.unwrap();
    assert_eq!(std::fs::read(target).unwrap(), b"download");
    assert_eq!(std::fs::read(unrelated).unwrap(), b"keep");
}

#[tokio::test]
async fn rename_failure_cannot_report_completed_or_destroy_an_external_directory() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("file.bin");
    let mut output = OutputFile::prepare(&target, &ReplacementPolicy::Deny)
        .await
        .unwrap();
    output.write(b"recover").await.unwrap();
    std::fs::create_dir(&target).unwrap();
    let error = output.finalize().await.unwrap_err();
    assert!(target.is_dir());
    assert_eq!(
        std::fs::read(error.retained_partial.unwrap()).unwrap(),
        b"recover"
    );
}
