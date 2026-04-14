use phronesis::bootstrap;
use phronesis::fs::meta;
use tempfile::TempDir;

/// AC-12: .meta.jsonl is append-only, last-line-wins
#[test]
fn ac12_meta_append_only_and_last_line_wins() {
    let tmp = TempDir::new().unwrap();
    bootstrap::bootstrap(tmp.path()).unwrap();

    let folder = tmp.path().join("praxis");

    // Read original seed meta
    let original_entries = meta::read_meta(&folder).unwrap();
    assert_eq!(original_entries.len(), 1);
    let original_desc = &original_entries[0].description;

    // Append a new description
    meta::append_meta(&folder, "updated praxis description").unwrap();

    // Both entries should exist (original NOT overwritten)
    let entries = meta::read_meta(&folder).unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(&entries[0].description, original_desc);
    assert_eq!(entries[1].description, "updated praxis description");

    // Last-line-wins: get_latest_description returns the second entry
    let latest = meta::get_latest_description(&folder).unwrap();
    assert_eq!(latest, Some("updated praxis description".to_string()));
}
