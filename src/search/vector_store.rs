use std::path::Path;

use instant_distance::{Builder, HnswMap, Search};
use serde::{Deserialize, Serialize};

use crate::error::{PhronesisError, Result};

/// A point in the vector space, wrapping a Vec<f32>.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VectorPoint(pub Vec<f32>);

impl instant_distance::Point for VectorPoint {
    fn distance(&self, other: &Self) -> f32 {
        // Cosine distance = 1 - cosine_similarity
        let dot: f32 = self.0.iter().zip(other.0.iter()).map(|(a, b)| a * b).sum();
        let norm_a: f32 = self.0.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = other.0.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm_a == 0.0 || norm_b == 0.0 {
            return 1.0;
        }
        1.0 - (dot / (norm_a * norm_b))
    }
}

/// Metadata for each indexed folder.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexEntry {
    pub path: String,
    pub description: String,
}

/// HNSW-based vector store using instant-distance.
pub struct HnswStore {
    /// Map from folder path to its vector and metadata
    entries: Vec<IndexEntry>,
    vectors: Vec<VectorPoint>,
    /// The HNSW index (rebuilt when entries change)
    hnsw: Option<HnswMap<VectorPoint, usize>>,
    /// Path to persist the index
    index_dir: std::path::PathBuf,
}

impl HnswStore {
    pub fn new(index_dir: &Path) -> Self {
        Self {
            entries: Vec::new(),
            vectors: Vec::new(),
            hnsw: None,
            index_dir: index_dir.to_path_buf(),
        }
    }

    /// Insert a folder into the index.
    pub fn insert(&mut self, path: String, description: String, vector: Vec<f32>) {
        // Check if already exists, update if so
        if let Some(idx) = self.entries.iter().position(|e| e.path == path) {
            self.entries[idx].description = description;
            self.vectors[idx] = VectorPoint(vector);
        } else {
            self.entries.push(IndexEntry { path, description });
            self.vectors.push(VectorPoint(vector));
        }
        self.rebuild_hnsw();
    }

    /// Remove a folder from the index by path.
    pub fn remove(&mut self, path: &str) -> bool {
        if let Some(idx) = self.entries.iter().position(|e| e.path == path) {
            self.entries.remove(idx);
            self.vectors.remove(idx);
            self.rebuild_hnsw();
            true
        } else {
            false
        }
    }

    /// Search for the top-k most similar folders.
    pub fn search(&self, query_vec: &[f32], top_k: usize) -> Vec<(IndexEntry, f32)> {
        let hnsw = match &self.hnsw {
            Some(h) => h,
            None => return Vec::new(),
        };

        if self.entries.is_empty() {
            return Vec::new();
        }

        let query = VectorPoint(query_vec.to_vec());
        let mut search = Search::default();
        let results: Vec<_> = hnsw.search(&query, &mut search).take(top_k).collect();

        results
            .into_iter()
            .filter_map(|item| {
                let idx = *item.value;
                self.entries
                    .get(idx)
                    .map(|entry| (entry.clone(), item.distance))
            })
            .collect()
    }

    /// Get count of indexed entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Save index to disk.
    pub fn save(&self) -> Result<()> {
        std::fs::create_dir_all(&self.index_dir)?;

        // Save metadata
        let meta_path = self.index_dir.join("metadata.jsonl");
        let mut meta_content = String::new();
        for entry in &self.entries {
            let line = serde_json::to_string(entry)?;
            meta_content.push_str(&line);
            meta_content.push('\n');
        }
        // Atomic write: write to temp, then rename
        let tmp_path = self.index_dir.join("metadata.jsonl.tmp");
        std::fs::write(&tmp_path, &meta_content)?;
        std::fs::rename(&tmp_path, &meta_path)?;

        // Save vectors
        let vectors_path = self.index_dir.join("vectors.json");
        let vectors_data: Vec<&[f32]> = self.vectors.iter().map(|v| v.0.as_slice()).collect();
        let json = serde_json::to_string(&vectors_data)?;
        let tmp_path = self.index_dir.join("vectors.json.tmp");
        std::fs::write(&tmp_path, &json)?;
        std::fs::rename(&tmp_path, &vectors_path)?;

        Ok(())
    }

    /// Load index from disk.
    pub fn load(index_dir: &Path) -> Result<Self> {
        let meta_path = index_dir.join("metadata.jsonl");
        let vectors_path = index_dir.join("vectors.json");

        if !meta_path.exists() || !vectors_path.exists() {
            return Ok(Self::new(index_dir));
        }

        // Load metadata
        let meta_content = std::fs::read_to_string(&meta_path)?;
        let mut entries = Vec::new();
        for (line_num, line) in meta_content.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            match serde_json::from_str::<IndexEntry>(trimmed) {
                Ok(entry) => entries.push(entry),
                Err(err) => {
                    tracing::warn!(
                        "Skipping corrupt JSONL line {} in metadata: {}",
                        line_num + 1,
                        err
                    );
                }
            }
        }

        // Load vectors
        let vectors_json = std::fs::read_to_string(&vectors_path)?;
        let raw_vectors: Vec<Vec<f32>> = serde_json::from_str(&vectors_json)
            .map_err(|e| PhronesisError::Embedding(format!("Failed to load vectors: {}", e)))?;
        let vectors: Vec<VectorPoint> = raw_vectors.into_iter().map(VectorPoint).collect();

        if entries.len() != vectors.len() {
            tracing::warn!(
                "Metadata/vector count mismatch ({} vs {}), rebuilding from metadata",
                entries.len(),
                vectors.len()
            );
            return Ok(Self::new(index_dir));
        }

        let mut store = Self {
            entries,
            vectors,
            hnsw: None,
            index_dir: index_dir.to_path_buf(),
        };
        store.rebuild_hnsw();
        Ok(store)
    }

    fn rebuild_hnsw(&mut self) {
        if self.vectors.is_empty() {
            self.hnsw = None;
            return;
        }
        let indices: Vec<usize> = (0..self.vectors.len()).collect();
        let hnsw = Builder::default().build(self.vectors.clone(), indices);
        self.hnsw = Some(hnsw);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_vec(seed: u64, dims: usize) -> Vec<f32> {
        let mut vec = Vec::with_capacity(dims);
        let mut state = seed;
        for _ in 0..dims {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            vec.push(((state >> 33) as f32) / (u32::MAX as f32));
        }
        vec
    }

    #[test]
    fn test_insert_and_search() {
        let tmp = TempDir::new().unwrap();
        let mut store = HnswStore::new(tmp.path());

        store.insert("self".into(), "self management".into(), make_vec(1, 8));
        store.insert("praxis".into(), "external actions".into(), make_vec(2, 8));
        store.insert(
            "cognition".into(),
            "internal reasoning".into(),
            make_vec(3, 8),
        );

        assert_eq!(store.len(), 3);

        let results = store.search(&make_vec(1, 8), 2);
        assert!(!results.is_empty());
        assert_eq!(results[0].0.path, "self"); // Exact match should be top
    }

    #[test]
    fn test_remove() {
        let tmp = TempDir::new().unwrap();
        let mut store = HnswStore::new(tmp.path());
        store.insert("a".into(), "desc".into(), make_vec(1, 8));
        store.insert("b".into(), "desc".into(), make_vec(2, 8));
        assert_eq!(store.len(), 2);

        store.remove("a");
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn test_save_and_load() {
        let tmp = TempDir::new().unwrap();
        let index_dir = tmp.path().join(".index");

        let mut store = HnswStore::new(&index_dir);
        store.insert("self".into(), "self management".into(), make_vec(1, 8));
        store.insert("praxis".into(), "external actions".into(), make_vec(2, 8));
        store.save().unwrap();

        let loaded = HnswStore::load(&index_dir).unwrap();
        assert_eq!(loaded.len(), 2);

        let results = loaded.search(&make_vec(1, 8), 1);
        assert_eq!(results[0].0.path, "self");
    }

    #[test]
    fn test_search_empty() {
        let tmp = TempDir::new().unwrap();
        let store = HnswStore::new(tmp.path());
        let results = store.search(&make_vec(1, 8), 5);
        assert!(results.is_empty());
    }

    #[test]
    fn test_update_existing() {
        let tmp = TempDir::new().unwrap();
        let mut store = HnswStore::new(tmp.path());
        store.insert("a".into(), "old desc".into(), make_vec(1, 8));
        store.insert("a".into(), "new desc".into(), make_vec(2, 8));
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn test_bulk_insert_and_search() {
        let tmp = TempDir::new().unwrap();
        let mut store = HnswStore::new(tmp.path());

        for i in 0..100 {
            store.insert(
                format!("folder_{}", i),
                format!("description {}", i),
                make_vec(i, 16),
            );
        }
        assert_eq!(store.len(), 100);

        let results = store.search(&make_vec(50, 16), 5);
        assert!(!results.is_empty());
        assert!(results.len() <= 5);
        assert_eq!(results[0].0.path, "folder_50");
    }

    #[test]
    fn test_save_load_integrity() {
        let tmp = TempDir::new().unwrap();
        let index_dir = tmp.path().join(".index");

        let mut store = HnswStore::new(&index_dir);
        for i in 0..20 {
            store.insert(format!("path_{}", i), format!("desc_{}", i), make_vec(i, 8));
        }
        store.save().unwrap();

        let loaded = HnswStore::load(&index_dir).unwrap();
        assert_eq!(loaded.len(), 20);

        for i in 0..20 {
            let results = loaded.search(&make_vec(i, 8), 1);
            assert_eq!(
                results[0].0.path,
                format!("path_{}", i),
                "Loaded store should find exact match for path_{}",
                i
            );
        }
    }

    #[test]
    fn test_remove_nonexistent() {
        let tmp = TempDir::new().unwrap();
        let mut store = HnswStore::new(tmp.path());
        store.insert("a".into(), "desc".into(), make_vec(1, 8));
        let removed = store.remove("nonexistent");
        assert!(!removed);
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn test_remove_all_entries() {
        let tmp = TempDir::new().unwrap();
        let mut store = HnswStore::new(tmp.path());
        store.insert("a".into(), "desc a".into(), make_vec(1, 8));
        store.insert("b".into(), "desc b".into(), make_vec(2, 8));

        store.remove("a");
        store.remove("b");

        assert!(store.is_empty());
        let results = store.search(&make_vec(1, 8), 5);
        assert!(results.is_empty());
    }

    #[test]
    fn test_load_empty_index() {
        let tmp = TempDir::new().unwrap();
        let index_dir = tmp.path().join(".nonexistent_index");
        let store = HnswStore::load(&index_dir).unwrap();
        assert!(store.is_empty());
    }

    #[test]
    fn test_cosine_distance_self_is_zero() {
        use instant_distance::Point;
        let v = VectorPoint(vec![1.0, 0.0, 0.0]);
        let dist = v.distance(&v);
        assert!(
            dist.abs() < 0.001,
            "Self-distance should be ~0, got {}",
            dist
        );
    }

    #[test]
    fn test_cosine_distance_orthogonal() {
        use instant_distance::Point;
        let v1 = VectorPoint(vec![1.0, 0.0]);
        let v2 = VectorPoint(vec![0.0, 1.0]);
        let dist = v1.distance(&v2);
        assert!(
            (dist - 1.0).abs() < 0.001,
            "Orthogonal distance should be ~1, got {}",
            dist
        );
    }

    #[test]
    fn test_cosine_distance_zero_vector() {
        use instant_distance::Point;
        let zero = VectorPoint(vec![0.0, 0.0, 0.0]);
        let v = VectorPoint(vec![1.0, 2.0, 3.0]);
        let dist = zero.distance(&v);
        assert_eq!(dist, 1.0, "Zero vector distance should be 1.0");
    }
}
