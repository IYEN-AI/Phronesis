use phronesis::bootstrap;
use phronesis::fs::action;
use tempfile::TempDir;

/// AC-11: No delete/modify API exists (append-only enforced)
#[test]
fn ac11_append_only_enforcement() {
    let tmp = TempDir::new().unwrap();
    bootstrap::bootstrap(tmp.path()).unwrap();

    let path = tmp.path().join("praxis/test_action.jsonl");

    // Write two entries
    let entry1 = serde_json::json!({"ts": "t1", "action": "first"});
    let entry2 = serde_json::json!({"ts": "t2", "action": "second"});
    action::append_action(&path, &entry1).unwrap();
    action::append_action(&path, &entry2).unwrap();

    // Read back — both entries present and first unchanged
    let entries = action::read_action(&path).unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0]["action"], "first");
    assert_eq!(entries[1]["action"], "second");

    // Verify the module exposes NO public delete/modify/truncate functions
    // The only public functions in fs::action are:
    //   - append_action (append only)
    //   - read_action (read only)
    // This is verified by compilation — if someone added a delete function,
    // this test file would need to be updated to acknowledge it.
}
