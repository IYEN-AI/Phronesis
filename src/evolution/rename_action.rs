use std::path::Path;

use crate::error::{PhronesisError, Result};
use crate::fs::naming;
use crate::fs::warnings;
use crate::types::RenameResult;

/// Rename an action file in-place. Validates naming and logs warning if violated.
/// No index change needed for file rename (index is folder-level).
pub fn rename_action(data_root: &Path, file_path: &str, new_name: &str) -> Result<RenameResult> {
    let full_path = data_root.join(file_path);
    if !full_path.exists() {
        return Err(PhronesisError::NotFound(format!(
            "File not found: {}",
            file_path
        )));
    }

    let parent = full_path
        .parent()
        .ok_or_else(|| PhronesisError::Validation("Cannot rename root".into()))?;

    let new_full = parent.join(new_name);
    std::fs::rename(&full_path, &new_full)?;

    // Validate naming convention
    let new_path_str = new_full
        .strip_prefix(data_root)
        .unwrap_or(&new_full)
        .to_string_lossy()
        .to_string();
    let warning = naming::validate_name(new_name, &new_path_str);

    // Log warning if naming violated
    if let Some(ref w) = warning {
        warnings::log_warning(data_root, w)?;
    }

    let old_name = full_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();

    Ok(RenameResult {
        old_name,
        new_name: new_name.to_string(),
        warning,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_rename_valid() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("praxis");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("old_name.jsonl"), "{\"ts\":\"t1\"}\n").unwrap();

        // Create warnings dir
        std::fs::create_dir_all(tmp.path().join("reflection/warnings")).unwrap();

        let result = rename_action(
            tmp.path(),
            "praxis/old_name.jsonl",
            "send_email_politely.jsonl",
        )
        .unwrap();

        assert_eq!(result.old_name, "old_name.jsonl");
        assert_eq!(result.new_name, "send_email_politely.jsonl");
        assert!(result.warning.is_none()); // Valid name
        assert!(dir.join("send_email_politely.jsonl").exists());
        assert!(!dir.join("old_name.jsonl").exists());
    }

    #[test]
    fn test_rename_invalid_name_logs_warning() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("praxis");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("old.jsonl"), "{\"ts\":\"t1\"}\n").unwrap();

        let result = rename_action(tmp.path(), "praxis/old.jsonl", "BAD NAME.txt").unwrap();

        assert!(result.warning.is_some());
        assert_eq!(result.warning.unwrap().rule_violated, "naming_convention");

        // Warning should be logged
        let warnings = warnings::get_warnings(tmp.path(), None).unwrap();
        assert_eq!(warnings.len(), 1);
    }

    #[test]
    fn test_rename_nonexistent() {
        let tmp = TempDir::new().unwrap();
        let result = rename_action(tmp.path(), "nope.jsonl", "new.jsonl");
        assert!(result.is_err());
    }

    #[test]
    fn test_rename_preserves_content() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("praxis");
        std::fs::create_dir_all(&dir).unwrap();

        let content = "{\"ts\":\"t1\",\"data\":\"keep\"}\n{\"ts\":\"t2\",\"data\":\"this\"}\n";
        std::fs::write(dir.join("old_action.jsonl"), content).unwrap();

        std::fs::create_dir_all(tmp.path().join("reflection/warnings")).unwrap();
        rename_action(
            tmp.path(),
            "praxis/old_action.jsonl",
            "keep_this_data.jsonl",
        )
        .unwrap();

        let new_content = std::fs::read_to_string(dir.join("keep_this_data.jsonl")).unwrap();
        assert_eq!(new_content, content);
    }

    #[test]
    fn test_rename_to_same_name() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("praxis");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("send_email.jsonl"), "{\"ts\":\"t1\"}\n").unwrap();

        std::fs::create_dir_all(tmp.path().join("reflection/warnings")).unwrap();
        let result = rename_action(tmp.path(), "praxis/send_email.jsonl", "send_email.jsonl");
        assert!(result.is_ok());
        assert!(dir.join("send_email.jsonl").exists());
    }
}
