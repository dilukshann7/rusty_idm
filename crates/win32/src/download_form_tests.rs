use super::*;

#[test]
fn derives_a_safe_filename_from_the_url_path_without_query_or_fragment() {
    assert_eq!(
        suggested_filename("https://example.com/files/report%20final.pdf?token=private#part")
            .unwrap(),
        "report final.pdf"
    );
    assert_eq!(
        suggested_filename("https://example.com/%E6%97%A5%E6%9C%AC%E8%AA%9E.zip").unwrap(),
        "日本語.zip"
    );
    assert_eq!(
        suggested_filename("https://example.com/").unwrap(),
        "download.bin"
    );
    assert_eq!(
        suggested_filename("https://example.com/CON.txt").unwrap(),
        "download.bin"
    );
    assert_eq!(
        suggested_filename("https://example.com/a%2Fb.zip").unwrap(),
        "a_b.zip"
    );
    assert!(suggested_filename("file:///C:/file.bin").is_err());
}

#[test]
fn remembers_only_the_selected_category_folder_and_uses_it_next_time() {
    let mut settings = crate::settings::Settings {
        schema_version: 1,
        default_destination: "C:/Downloads".into(),
        category_destinations: Default::default(),
        duplicate_choice: None,
        show_completion_dialog: true,
    };
    remember_folder(
        &mut settings,
        "Documents",
        std::path::Path::new("D:/Papers/report.pdf"),
    );
    assert_eq!(
        folder_for(&settings, "Documents"),
        std::path::Path::new("D:/Papers")
    );
    assert_eq!(
        folder_for(&settings, "General"),
        std::path::Path::new("C:/Downloads")
    );
}

#[test]
fn browser_filename_hints_cannot_supply_paths_or_reserved_names() {
    let url = "https://example.org/safe.bin?token=private";
    assert_eq!(
        filename_hint(url, Some("C:\\escape\\report.pdf")).unwrap(),
        "report.pdf"
    );
    assert_eq!(
        filename_hint(url, Some("../../report.pdf")).unwrap(),
        "report.pdf"
    );
    assert_eq!(filename_hint(url, Some("CON.txt")).unwrap(), "safe.bin");
    assert_eq!(
        filename_hint(url, Some("bad\nname.zip")).unwrap(),
        "bad_name.zip"
    );
}
