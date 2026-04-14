use phronesis::bootstrap;
use phronesis::fs::action;
use phronesis::search::grep;
use tempfile::TempDir;

/// AC-2: grep_search finds existing action files by name + content
#[test]
fn ac02_grep_finds_by_name_and_content() {
    let tmp = TempDir::new().unwrap();
    bootstrap::bootstrap(tmp.path()).unwrap();

    // Create action file with known content
    let file_path = tmp.path().join("praxis/send_apology_email.jsonl");
    let entry = serde_json::json!({
        "ts": "2026-04-14T10:00:00Z",
        "situation": "사용자가 불만을 표시함",
        "action": "공감 표현 후 대안 제시"
    });
    action::append_action(&file_path, &entry).unwrap();

    // Find by filename pattern
    let results = grep::grep_search(tmp.path().join("praxis").as_path(), "apology", None).unwrap();
    assert!(!results.is_empty());
    assert!(results.iter().any(|r| r.matched_lines.iter().any(|m| m.is_filename_match)));

    // Find by content pattern
    let results = grep::grep_search(tmp.path().join("praxis").as_path(), "공감", None).unwrap();
    assert!(!results.is_empty());
    assert!(results.iter().any(|r| r.matched_lines.iter().any(|m| !m.is_filename_match)));

    // Verify max_results default caps at 50
    let results = grep::grep_search(tmp.path().join("praxis").as_path(), ".", None).unwrap();
    assert!(results.len() <= 50);

    // Verify explicit max_results
    let results = grep::grep_search(tmp.path().join("praxis").as_path(), ".", Some(2)).unwrap();
    assert!(results.len() <= 2);
}
