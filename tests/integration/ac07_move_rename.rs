use phronesis::bootstrap;
use phronesis::evolution::{move_action, rename_action};
use phronesis::fs::{action, meta};
use phronesis::search::embedding::{EmbeddingProvider, MockEmbeddingProvider};
use phronesis::search::vector_store::HnswStore;
use tempfile::TempDir;

/// AC-7: move/rename auto-updates embedding index
#[tokio::test]
async fn ac07_move_folder_updates_index() {
    let tmp = TempDir::new().unwrap();
    bootstrap::bootstrap(tmp.path()).unwrap();

    let provider = MockEmbeddingProvider::new(8);
    let mut store = HnswStore::new(&tmp.path().join(".index"));

    // Create and index a subfolder
    let sub = tmp.path().join("praxis/old_communication");
    std::fs::create_dir_all(&sub).unwrap();
    meta::append_meta(&sub, "커뮤니케이션 행동").unwrap();

    let vec = provider.embed("커뮤니케이션 행동").await.unwrap();
    store.insert(
        "praxis/old_communication".into(),
        "커뮤니케이션 행동".into(),
        vec,
    );

    // Move folder
    move_action::move_action(
        tmp.path(),
        "praxis/old_communication",
        "praxis/new_communication",
        &mut store,
        &provider,
    )
    .await
    .unwrap();

    // Old path should NOT be found
    let query_vec = provider.embed("커뮤니케이션 행동").await.unwrap();
    let results = store.search(&query_vec, 5);
    assert!(results.iter().all(|r| r.0.path != "praxis/old_communication"));

    // New path SHOULD be found
    assert!(results.iter().any(|r| r.0.path == "praxis/new_communication"));
}

/// AC-7: file rename works
#[test]
fn ac07_rename_file() {
    let tmp = TempDir::new().unwrap();
    bootstrap::bootstrap(tmp.path()).unwrap();

    let dir = tmp.path().join("praxis");
    let file = dir.join("old_action.jsonl");
    action::append_action(&file, &serde_json::json!({"ts": "t1"})).unwrap();

    let result =
        rename_action::rename_action(tmp.path(), "praxis/old_action.jsonl", "send_reply_fast.jsonl")
            .unwrap();

    assert_eq!(result.new_name, "send_reply_fast.jsonl");
    assert!(result.warning.is_none()); // Valid name

    // Old file gone, new file exists
    assert!(!file.exists());
    assert!(dir.join("send_reply_fast.jsonl").exists());

    // Content preserved
    let entries = action::read_action(&dir.join("send_reply_fast.jsonl")).unwrap();
    assert_eq!(entries.len(), 1);
}
