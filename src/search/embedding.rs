use crate::error::Result;

/// Trait for embedding text into vectors. Abstracted for swappable providers.
#[async_trait::async_trait]
pub trait EmbeddingProvider: Send + Sync {
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;
    fn dimensions(&self) -> usize;
}

/// OpenAI text-embedding-3-small implementation.
pub struct OpenAIEmbedding {
    api_key: String,
    model: String,
    client: reqwest::Client,
}

impl OpenAIEmbedding {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            api_key,
            model,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait::async_trait]
impl EmbeddingProvider for OpenAIEmbedding {
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let body = serde_json::json!({
            "input": text,
            "model": &self.model,
        });

        let resp = self
            .client
            .post("https://api.openai.com/v1/embeddings")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await
            .map_err(|e| crate::error::PhronesisError::Embedding(e.to_string()))?;

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| crate::error::PhronesisError::Embedding(e.to_string()))?;

        let embedding = json["data"][0]["embedding"]
            .as_array()
            .ok_or_else(|| {
                crate::error::PhronesisError::Embedding("Invalid embedding response".into())
            })?
            .iter()
            .filter_map(|v| v.as_f64().map(|f| f as f32))
            .collect();

        Ok(embedding)
    }

    fn dimensions(&self) -> usize {
        1536 // text-embedding-3-small default
    }
}

/// Local embedding provider using fastembed (ONNX, no API key needed).
/// Uses MultilingualE5Small model (384 dimensions, 100+ languages including Korean).
/// Model is downloaded automatically on first use (~100MB).
pub struct LocalEmbedding {
    model: std::sync::Mutex<fastembed::TextEmbedding>,
}

impl LocalEmbedding {
    pub fn new() -> Result<Self> {
        let options = fastembed::InitOptions::new(fastembed::EmbeddingModel::MultilingualE5Small)
            .with_show_download_progress(true);
        let model = fastembed::TextEmbedding::try_new(options)
            .map_err(|e| crate::error::PhronesisError::Embedding(format!("Failed to init local embedding model: {}", e)))?;
        Ok(Self {
            model: std::sync::Mutex::new(model),
        })
    }
}

#[async_trait::async_trait]
impl EmbeddingProvider for LocalEmbedding {
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let text = text.to_string();
        let mut model = self.model.lock()
            .map_err(|e| crate::error::PhronesisError::Embedding(format!("Lock error: {}", e)))?;
        let embeddings = model.embed(vec![text], None)
            .map_err(|e| crate::error::PhronesisError::Embedding(e.to_string()))?;
        embeddings.into_iter().next()
            .ok_or_else(|| crate::error::PhronesisError::Embedding("No embedding returned".into()))
    }

    fn dimensions(&self) -> usize {
        384 // MultilingualE5Small
    }
}

/// Mock embedding provider for testing. Uses hash-based deterministic vectors.
#[cfg(any(test, feature = "test-utils"))]
pub struct MockEmbeddingProvider {
    dims: usize,
}

#[cfg(any(test, feature = "test-utils"))]
impl MockEmbeddingProvider {
    pub fn new(dims: usize) -> Self {
        Self { dims }
    }
}

#[cfg(any(test, feature = "test-utils"))]
#[async_trait::async_trait]
impl EmbeddingProvider for MockEmbeddingProvider {
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        let seed = hasher.finish();

        let mut vec = Vec::with_capacity(self.dims);
        let mut state = seed;
        for _ in 0..self.dims {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            let val = ((state >> 33) as f32) / (u32::MAX as f32) * 2.0 - 1.0;
            vec.push(val);
        }

        // Normalize
        let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for v in &mut vec {
                *v /= norm;
            }
        }

        Ok(vec)
    }

    fn dimensions(&self) -> usize {
        self.dims
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_embedding_deterministic() {
        let provider = MockEmbeddingProvider::new(8);
        let v1 = provider.embed("hello").await.unwrap();
        let v2 = provider.embed("hello").await.unwrap();
        assert_eq!(v1, v2);
    }

    #[tokio::test]
    async fn test_mock_embedding_different_inputs() {
        let provider = MockEmbeddingProvider::new(8);
        let v1 = provider.embed("hello").await.unwrap();
        let v2 = provider.embed("world").await.unwrap();
        assert_ne!(v1, v2);
    }

    #[tokio::test]
    async fn test_mock_embedding_normalized() {
        let provider = MockEmbeddingProvider::new(16);
        let v = provider.embed("test").await.unwrap();
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.01);
    }
}
