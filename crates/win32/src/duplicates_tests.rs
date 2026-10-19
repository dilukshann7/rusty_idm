use super::*;

#[test]
fn numbered_names_skip_files_and_reserved_destinations_and_preserve_extension() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().join("日本語.tar.gz");
    std::fs::write(dir.path().join("日本語.tar (1).gz"), b"old").unwrap();
    let reserved = vec![dir.path().join("日本語.tar (2).gz")];
    assert_eq!(
        numbered_destination(&base, &reserved).unwrap(),
        dir.path().join("日本語.tar (3).gz")
    );
    assert_eq!(
        std::fs::read(dir.path().join("日本語.tar (1).gz")).unwrap(),
        b"old"
    );
    assert_eq!(
        numbered_destination(&dir.path().join("file"), &[]).unwrap(),
        dir.path().join("file (1)")
    );
}

#[test]
fn destination_comparison_normalizes_windows_separator_and_case() {
    assert!(same_destination(
        Path::new("C:/Downloads/File.ZIP"),
        Path::new("c:\\downloads\\file.zip")
    ));
    assert!(!same_destination(
        Path::new("C:/Downloads/file.zip"),
        Path::new("C:/Downloads/file (1).zip")
    ));
}
