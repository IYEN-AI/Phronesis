use std::path::Path;

use crate::error::{PhronesisError, Result};
use crate::fs::meta;
use crate::search::embedding::EmbeddingProvider;
use crate::search::vector_store::HnswStore;
use crate::types::MoveResult;

/// Move a file or folder, updating the embedding index if a folder is moved.
pub async fn move_action<P: EmbeddingProvider>(
    data_root: &Path,
    old_path: &str,
    new_path: &str,
    store: &mut HnswStore,
    provider: &P,
) -> Result<MoveResult> {
    let old_full = data_root.join(old_path);
    let new_full = data_root.join(new_path);

    if !old_full.exists() {
        return Err(PhronesisError::NotFound(format!(
            "Source not found: {}",
            old_path
        )));
    }

    // Ensure parent directory of new path exists
    if let Some(parent) = new_full.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let is_dir = old_full.is_dir();

    // Perform the move
    std::fs::rename(&old_full, &new_full)?;

    // If folder moved, update embedding index
    if is_dir {
        // Remove old path from index
        store.remove(old_path);

        // Re-embed with new path's .meta.jsonl description
        if let Some(description) = meta::get_latest_description(&new_full)? {
            let vector = provider.embed(&description).await?;
            store.insert(new_path.to_string(), description, vector);
        }
    }

    Ok(MoveResult {
        old_path: old_path.to_string(),
        new_path: new_path.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::embedding::MockEmbeddingProvider;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_move_folder_updates_index() {
        let tmp = TempDir::new().unwrap();
        let provider = MockEmbeddingProvider::new(8);
        let mut store = HnswStore::new(&tmp.path().join(".index"));

        // Create source folder with meta
        let src = tmp.path().join("old_folder");
        std::fs::create_dir_all(&src).unwrap();
        meta::append_meta(&src, "test folder description").unwrap();

        // Index it
        let vec = provider.embed("test folder description").await.unwrap();
        store.insert("old_folder".into(), "test folder description".into(), vec);
        assert_eq!(store.len(), 1);

        // Move it
        let result = move_action(
            tmp.path(),
            "old_folder",
            "new_folder",
            &mut store,
            &provider,
        )
        .await
        .unwrap();

        assert_eq!(result.old_path, "old_folder");
        assert_eq!(result.new_path, "new_folder");

        // Index should have new path, not old
        assert_eq!(store.len(), 1);
        let search_results = store.search(
            &provider.embed("test folder description").await.unwrap(),
            1,
        );
        assert_eq!(search_results[0].0.path, "new_folder");
    }

    #[tokio::test]
    async fn test_move_file() {
        let tmp = TempDir::new().unwrap();
        let provider = MockEmbeddingProvider::new(8);
        let mut store = HnswStore::new(&tmp.path().join(".index"));

        // Create a file
        let dir = tmp.path().join("folder");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("action.jsonl"), "{\"ts\":\"t1\"}\n").unwrap();

        let result = move_action(
            tmp.path(),
            "folder/action.jsonl",
            "folder/moved_action.jsonl",
            &mut store,
            &provider,
        )
        .await
        .unwrap();

        assert!(!tmp.path().join("folder/action.jsonl").exists());
        assert!(tmp.path().join("folder/moved_action.jsonl").exists());
        assert_eq!(result.new_path, "folder/moved_action.jsonl");
    }

    #[tokio::test]
    async fn test_move_nonexistent() {
        let tmp = TempDir::new().unwrap();
        let provider = MockEmbeddingProvider::new(8);
        let mut store = HnswStore::new(&tmp.path().join(".index"));

        let result = move_action(tmp.path(), "nope", "dest", &mut store, &provider).await;
        assert!(result.is_err());
    }
}
