use phronesis::bootstrap;
use phronesis::fs::{action, meta};
use phronesis::search::embedding::{EmbeddingProvider, MockEmbeddingProvider};
use phronesis::search::{grep, vector_store::HnswStore};
use tempfile::TempDir;

/// AC-5: Previously created action files are re-discoverable in subsequent sessions
#[tokio::test]
async fn ac05_cross_session_persistence() {
    let tmp = TempDir::new().unwrap();
    let provider = MockEmbeddingProvider::new(8);

    // === Session 1: Create and index ===
    bootstrap::bootstrap(tmp.path()).unwrap();

    let mut store = HnswStore::new(&tmp.path().join(".index"));

    // Index pillars
    for name in ["self", "perception", "cognition", "praxis", "evolution", "reflection"] {
        let desc = meta::get_latest_description(&tmp.path().join(name))
            .unwrap()
            .unwrap();
        let vec = provider.embed(&desc).await.unwrap();
        store.insert(name.to_string(), desc, vec);
    }

    // Write an action file
    let action_path = tmp.path().join("praxis/handle_complaint_calmly.jsonl");
    let entry = serde_json::json!({
        "ts": "2026-04-14T10:00:00Z",
        "action": "침착하게 불만 처리"
    });
    action::append_action(&action_path, &entry).unwrap();

    // Save index
    store.save().unwrap();
    drop(store); // End "session 1"

    // === Session 2: Load and find ===
    let store2 = HnswStore::load(&tmp.path().join(".index")).unwrap();
    assert!(!store2.is_empty());

    // grep_search finds the file
    let grep_results =
        grep::grep_search(&tmp.path().join("praxis"), "complaint", None).unwrap();
    assert!(!grep_results.is_empty());

    // embed_search finds the folder
    let query_vec = provider
        .embed("외부 세계에 영향을 미치는 실천 행동")
        .await
        .unwrap();
    let embed_results = store2.search(&query_vec, 3);
    assert!(!embed_results.is_empty());
    assert!(embed_results.iter().any(|r| r.0.path == "praxis"));
}
