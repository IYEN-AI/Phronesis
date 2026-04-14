use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

use crate::error::Result;
use crate::types::MetaEntry;

/// Read all valid meta entries from a folder's .meta.jsonl.
/// Skips unparseable lines with a tracing warning (defensive parsing).
pub fn read_meta(folder: &Path) -> Result<Vec<MetaEntry>> {
    let meta_path = folder.join(".meta.jsonl");
    if !meta_path.exists() {
        return Ok(Vec::new());
    }

    let file = std::fs::File::open(&meta_path)?;
    let reader = BufReader::new(file);
    let mut entries = Vec::new();

    for (line_num, line) in reader.lines().enumerate() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match serde_json::from_str::<MetaEntry>(trimmed) {
            Ok(entry) => entries.push(entry),
            Err(err) => {
                tracing::warn!(
                    "Skipping corrupt JSONL line {} in {}: {}",
                    line_num + 1,
                    meta_path.display(),
                    err
                );
            }
        }
    }

    Ok(entries)
}

/// Get the latest (last valid line) description for a folder.
/// "Last line wins" semantics.
pub fn get_latest_description(folder: &Path) -> Result<Option<String>> {
    let entries = read_meta(folder)?;
    Ok(entries.last().map(|e| e.description.clone()))
}

/// Append a new meta entry to a folder's .meta.jsonl.
pub fn append_meta(folder: &Path, description: &str) -> Result<()> {
    let meta_path = folder.join(".meta.jsonl");
    let entry = MetaEntry {
        description: description.to_string(),
        created: None,
        updated: Some(chrono::Utc::now().to_rfc3339()),
    };
    let line = serde_json::to_string(&entry)?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&meta_path)?;
    writeln!(file, "{}", line)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_read_meta_empty_folder() {
        let tmp = TempDir::new().unwrap();
        let entries = read_meta(tmp.path()).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn test_append_and_read_meta() {
        let tmp = TempDir::new().unwrap();
        append_meta(tmp.path(), "first description").unwrap();
        append_meta(tmp.path(), "updated description").unwrap();

        let entries = read_meta(tmp.path()).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].description, "first description");
        assert_eq!(entries[1].description, "updated description");
    }

    #[test]
    fn test_last_line_wins() {
        let tmp = TempDir::new().unwrap();
        append_meta(tmp.path(), "old").unwrap();
        append_meta(tmp.path(), "new").unwrap();

        let desc = get_latest_description(tmp.path()).unwrap();
        assert_eq!(desc, Some("new".to_string()));
    }

    #[test]
    fn test_defensive_parsing() {
        let tmp = TempDir::new().unwrap();
        let meta_path = tmp.path().join(".meta.jsonl");
        std::fs::write(
            &meta_path,
            "{\"description\":\"good\"}\n{corrupt\n{\"description\":\"also good\"}\n",
        )
        .unwrap();

        let entries = read_meta(tmp.path()).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].description, "good");
        assert_eq!(entries[1].description, "also good");
    }

    #[test]
    fn test_meta_is_append_only() {
        let tmp = TempDir::new().unwrap();
        append_meta(tmp.path(), "first").unwrap();
        append_meta(tmp.path(), "second").unwrap();

        let entries = read_meta(tmp.path()).unwrap();
        // First entry is preserved, not overwritten
        assert_eq!(entries[0].description, "first");
        assert_eq!(entries.len(), 2);
    }
}
