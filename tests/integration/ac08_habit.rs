use phronesis::bootstrap;
use phronesis::evolution::habit;
use phronesis::fs::action;
use tempfile::TempDir;

/// AC-8: create_habit creates working symlink shortcuts
#[test]
#[cfg(unix)]
fn ac08_symlink_habit() {
    let tmp = TempDir::new().unwrap();
    bootstrap::bootstrap(tmp.path()).unwrap();

    // Create source action
    let source = tmp.path().join("praxis/send_apology_email.jsonl");
    action::append_action(
        &source,
        &serde_json::json!({"ts": "t1", "action": "send apology"}),
    )
    .unwrap();

    // Create habit shortcut
    habit::create_habit(
        tmp.path(),
        "praxis/send_apology_email.jsonl",
        "habits/quick_apology.jsonl",
    )
    .unwrap();

    // Verify shortcut is a symlink and readable
    let shortcut = tmp.path().join("habits/quick_apology.jsonl");
    assert!(shortcut.exists());
    assert!(shortcut.is_symlink());

    // Can read through symlink
    let entries = action::read_action(&shortcut).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["action"], "send apology");
}
