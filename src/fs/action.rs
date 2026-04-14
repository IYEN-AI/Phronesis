use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

use crate::error::{PhronesisError, Result};

/// Append a JSONL entry to an action file. Creates the file if absent.
/// The entry MUST contain a "ts" field (enforced by caller).
pub fn append_action(path: &Path, content: &serde_json::Value) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let line = serde_json::to_string(content)?;
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "{}", line)?;
    Ok(())
}

/// Read all valid JSONL entries from an action file.
/// Skips unparseable lines with a tracing warning (defensive parsing).
pub fn read_action(path: &Path) -> Result<Vec<serde_json::Value>> {
    if !path.exists() {
        return Err(PhronesisError::NotFound(format!(
            "Action file not found: {}",
            path.display()
        )));
    }

    let file = std::fs::File::open(path)?;
    let reader = BufReader::new(file);
    let mut entries = Vec::new();

    for (line_num, line) in reader.lines().enumerate() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match serde_json::from_str::<serde_json::Value>(trimmed) {
            Ok(value) => entries.push(value),
            Err(err) => {
                tracing::warn!(
                    "Skipping corrupt JSONL line {} in {}: {}",
                    line_num + 1,
                    path.display(),
                    err
                );
            }
        }
    }

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_append_and_read() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("test_action.jsonl");

        let entry1 = serde_json::json!({"ts": "2026-04-14T10:00:00Z", "action": "greet"});
        let entry2 = serde_json::json!({"ts": "2026-04-14T11:00:00Z", "action": "respond"});

        append_action(&path, &entry1).unwrap();
        append_action(&path, &entry2).unwrap();

        let entries = read_action(&path).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0]["action"], "greet");
        assert_eq!(entries[1]["action"], "respond");
    }

    #[test]
    fn test_read_nonexistent() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("nonexistent.jsonl");
        assert!(read_action(&path).is_err());
    }

    #[test]
    fn test_defensive_parsing_skips_corrupt_line() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("corrupt.jsonl");

        // Write valid line, then corrupt line, then valid line
        std::fs::write(
            &path,
            "{\"ts\":\"t1\",\"a\":1}\n{corrupt\n{\"ts\":\"t2\",\"a\":2}\n",
        )
        .unwrap();

        let entries = read_action(&path).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0]["a"], 1);
        assert_eq!(entries[1]["a"], 2);
    }

    #[test]
    fn test_append_only_preserves_existing_entries() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("verify_append.jsonl");

        let entry1 = serde_json::json!({"ts": "t1", "data": "first"});
        let entry2 = serde_json::json!({"ts": "t2", "data": "second"});
        let entry3 = serde_json::json!({"ts": "t3", "data": "third"});

        append_action(&path, &entry1).unwrap();
        append_action(&path, &entry2).unwrap();
        append_action(&path, &entry3).unwrap();

        let entries = read_action(&path).unwrap();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0]["data"], "first");
        assert_eq!(entries[1]["data"], "second");
        assert_eq!(entries[2]["data"], "third");

        let raw = std::fs::read_to_string(&path).unwrap();
        let line_count = raw.lines().filter(|l| !l.trim().is_empty()).count();
        assert_eq!(line_count, 3, "Each append should produce exactly one line");
    }

    #[test]
    fn test_append_creates_parent_directories() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("deep/nested/dir/action.jsonl");

        let entry = serde_json::json!({"ts": "t1", "action": "test"});
        append_action(&path, &entry).unwrap();

        assert!(path.exists());
        let entries = read_action(&path).unwrap();
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn test_read_empty_file() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("empty.jsonl");
        std::fs::write(&path, "").unwrap();

        let entries = read_action(&path).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn test_read_whitespace_only_file() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("whitespace.jsonl");
        std::fs::write(&path, "\n\n  \n\n").unwrap();

        let entries = read_action(&path).unwrap();
        assert!(entries.is_empty());
    }
}
