use std::path::Path;

use crate::error::{PhronesisError, Result};
use crate::types::HabitResult;

/// Create a symlink (habit) from shortcut to source.
/// This enables quick access to frequently used action files.
#[cfg(unix)]
pub fn create_habit(data_root: &Path, source: &str, shortcut: &str) -> Result<HabitResult> {
    let source_full = data_root.join(source);
    let shortcut_full = data_root.join(shortcut);

    if !source_full.exists() {
        return Err(PhronesisError::NotFound(format!(
            "Source not found: {}",
            source
        )));
    }

    // Ensure parent directory of shortcut exists
    if let Some(parent) = shortcut_full.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Create symlink
    std::os::unix::fs::symlink(&source_full, &shortcut_full)?;

    Ok(HabitResult {
        source: source.to_string(),
        shortcut: shortcut.to_string(),
    })
}

#[cfg(not(unix))]
pub fn create_habit(data_root: &Path, source: &str, shortcut: &str) -> Result<HabitResult> {
    Err(PhronesisError::Validation(
        "Symlink habits are only supported on Unix systems".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    #[cfg(unix)]
    fn test_create_habit() {
        let tmp = TempDir::new().unwrap();

        // Create source file
        let dir = tmp.path().join("praxis/communication");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("send_apology_email.jsonl"),
            "{\"ts\":\"t1\"}\n",
        )
        .unwrap();

        let result = create_habit(
            tmp.path(),
            "praxis/communication/send_apology_email.jsonl",
            "habits/quick_apology.jsonl",
        )
        .unwrap();

        assert_eq!(result.source, "praxis/communication/send_apology_email.jsonl");
        assert_eq!(result.shortcut, "habits/quick_apology.jsonl");

        // Verify symlink works
        let shortcut_path = tmp.path().join("habits/quick_apology.jsonl");
        assert!(shortcut_path.exists());
        assert!(shortcut_path.is_symlink());

        // Can read through symlink
        let content = std::fs::read_to_string(&shortcut_path).unwrap();
        assert!(content.contains("t1"));
    }

    #[test]
    #[cfg(unix)]
    fn test_create_habit_nonexistent_source() {
        let tmp = TempDir::new().unwrap();
        let result = create_habit(tmp.path(), "nonexistent.jsonl", "shortcut.jsonl");
        assert!(result.is_err());
    }
}
