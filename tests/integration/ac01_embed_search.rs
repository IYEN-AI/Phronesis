use phronesis::bootstrap;
use phronesis::search::embedding::MockEmbeddingProvider;
use phronesis::search::vector_store::HnswStore;
use phronesis::search::embedding::EmbeddingProvider;
use phronesis::fs::meta;
use tempfile::TempDir;

/// AC-1: embed_search returns correct folder in Top-K
#[tokio::test]
async fn ac01_embed_search_returns_correct_folder() {
    let tmp = TempDir::new().unwrap();
    bootstrap::bootstrap(tmp.path()).unwrap();

    let provider = MockEmbeddingProvider::new(8);
    let mut store = HnswStore::new(&tmp.path().join(".index"));

    // Create subfolders with distinct descriptions
    let sub1 = tmp.path().join("cognition/logical_reasoning");
    std::fs::create_dir_all(&sub1).unwrap();
    meta::append_meta(&sub1, "논리적 추론과 분석적 사고 프로세스").unwrap();

    let sub2 = tmp.path().join("praxis/communication/email");
    std::fs::create_dir_all(&sub2).unwrap();
    meta::append_meta(&sub2, "이메일을 통한 공식적 커뮤니케이션").unwrap();

    let sub3 = tmp.path().join("perception/user_intent");
    std::fs::create_dir_all(&sub3).unwrap();
    meta::append_meta(&sub3, "사용자의 의도와 감정 분석").unwrap();

    // Index all folders
    for (path, desc) in [
        ("cognition/logical_reasoning", "논리적 추론과 분석적 사고 프로세스"),
        ("praxis/communication/email", "이메일을 통한 공식적 커뮤니케이션"),
        ("perception/user_intent", "사용자의 의도와 감정 분석"),
    ] {
        let vec = provider.embed(desc).await.unwrap();
        store.insert(path.to_string(), desc.to_string(), vec);
    }

    // Search for "논리적 추론" should return cognition/logical_reasoning in top 3
    let query_vec = provider.embed("논리적 추론과 분석적 사고 프로세스").await.unwrap();
    let results = store.search(&query_vec, 3);

    assert!(!results.is_empty());
    assert_eq!(results[0].0.path, "cognition/logical_reasoning");
}
