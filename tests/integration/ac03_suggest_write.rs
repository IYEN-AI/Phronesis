use phronesis::bootstrap;
use phronesis::fs::{action, meta};
use phronesis::search::embedding::{EmbeddingProvider, MockEmbeddingProvider};
use phronesis::search::{suggest, vector_store::HnswStore};
use tempfile::TempDir;

/// AC-3: suggest_location -> write_action creates file at proper location with trajectory
#[tokio::test]
async fn ac03_suggest_and_write_cycle() {
    let tmp = TempDir::new().unwrap();
    bootstrap::bootstrap(tmp.path()).unwrap();

    let provider = MockEmbeddingProvider::new(8);
    let mut store = HnswStore::new(&tmp.path().join(".index"));

    // Index the 6 pillars
    for name in ["self", "perception", "cognition", "praxis", "evolution", "reflection"] {
        let desc = meta::get_latest_description(&tmp.path().join(name))
            .unwrap()
            .unwrap();
        let vec = provider.embed(&desc).await.unwrap();
        store.insert(name.to_string(), desc, vec);
    }

    // Suggest location for "외부 실천 행동"
    let suggestions =
        suggest::suggest_location("외부 세계에 영향을 미치는 실천 행동", &provider, &store, 3)
            .await
            .unwrap();
    assert!(!suggestions.is_empty());

    // Pick top candidate and write action there
    let target_folder = &suggestions[0].path;
    let action_path = tmp
        .path()
        .join(target_folder)
        .join("greet_user_warmly.jsonl");

    let entry = serde_json::json!({
        "ts": "2026-04-14T10:00:00Z",
        "situation": "새로운 사용자가 접속",
        "reasoning": "첫인상이 중요하므로 따뜻하게 인사",
        "action": "따뜻한 인사와 함께 도움 제안",
        "outcome": "사용자가 긍정적으로 반응"
    });
    action::append_action(&action_path, &entry).unwrap();

    // Verify file was created and content is readable
    let entries = action::read_action(&action_path).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["action"], "따뜻한 인사와 함께 도움 제안");
}
