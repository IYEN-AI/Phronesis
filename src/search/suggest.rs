use crate::error::Result;
use crate::search::embedding::EmbeddingProvider;
use crate::search::vector_store::HnswStore;
use crate::types::LocationCandidate;

/// Suggest the best folder locations for a new action based on description.
pub async fn suggest_location<P: EmbeddingProvider + ?Sized>(
    description: &str,
    provider: &P,
    store: &HnswStore,
    top_k: usize,
) -> Result<Vec<LocationCandidate>> {
    let query_vec = provider.embed(description).await?;
    let results = store.search(&query_vec, top_k);

    Ok(results
        .into_iter()
        .map(|(entry, score)| LocationCandidate {
            path: entry.path,
            description: entry.description,
            score: 1.0 - score, // Convert distance to similarity
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::embedding::MockEmbeddingProvider;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_suggest_location() {
        let tmp = TempDir::new().unwrap();
        let provider = MockEmbeddingProvider::new(8);
        let mut store = HnswStore::new(tmp.path());

        let v1 = provider.embed("self management").await.unwrap();
        let v2 = provider.embed("external actions and communication").await.unwrap();
        let v3 = provider.embed("internal reasoning and logic").await.unwrap();

        store.insert("self".into(), "self management".into(), v1);
        store.insert("praxis".into(), "external actions and communication".into(), v2);
        store.insert("cognition".into(), "internal reasoning and logic".into(), v3);

        let suggestions = suggest_location("reasoning about logic", &provider, &store, 3)
            .await
            .unwrap();

        assert!(!suggestions.is_empty());
        assert!(suggestions.len() <= 3);
    }

    #[tokio::test]
    async fn test_suggest_location_empty_store() {
        let tmp = TempDir::new().unwrap();
        let provider = MockEmbeddingProvider::new(8);
        let store = HnswStore::new(tmp.path());

        let suggestions = suggest_location("anything", &provider, &store, 3)
            .await
            .unwrap();
        assert!(suggestions.is_empty());
    }
}
