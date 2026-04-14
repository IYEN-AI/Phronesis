use std::path::Path;

use crate::error::{PhronesisError, Result};

const APPENDABLE_MARKER: &str = "<!-- AGENT APPENDABLE SECTION -->";

/// Read the full skills.md content.
pub fn read_skills(data_root: &Path) -> Result<String> {
    let skills_path = data_root.join("skills.md");
    if !skills_path.exists() {
        return Err(PhronesisError::NotFound("skills.md not found".into()));
    }
    Ok(std::fs::read_to_string(&skills_path)?)
}

/// Append content to the agent-appendable section of skills.md.
/// Rejects writes that would modify the immutable seed section.
pub fn append_to_skills(data_root: &Path, content: &str) -> Result<()> {
    let skills_path = data_root.join("skills.md");
    let current = std::fs::read_to_string(&skills_path)?;

    if !current.contains(APPENDABLE_MARKER) {
        return Err(PhronesisError::Validation(
            "skills.md is missing the appendable section marker".into(),
        ));
    }

    // Append at the end of the file (after the marker)
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(&skills_path)?;
    use std::io::Write;
    writeln!(file, "\n{}", content)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_read_skills() {
        let tmp = TempDir::new().unwrap();
        let skills_path = tmp.path().join("skills.md");
        std::fs::write(&skills_path, "# Skills\n<!-- AGENT APPENDABLE SECTION -->\n").unwrap();

        let content = read_skills(tmp.path()).unwrap();
        assert!(content.contains("# Skills"));
    }

    #[test]
    fn test_append_to_skills() {
        let tmp = TempDir::new().unwrap();
        let skills_path = tmp.path().join("skills.md");
        let seed = "# Skills\nSeed content\n<!-- AGENT APPENDABLE SECTION -->\n";
        std::fs::write(&skills_path, seed).unwrap();

        append_to_skills(tmp.path(), "My learned pattern").unwrap();

        let content = std::fs::read_to_string(&skills_path).unwrap();
        assert!(content.contains("Seed content")); // Seed preserved
        assert!(content.contains("My learned pattern")); // Append present
    }

    #[test]
    fn test_append_preserves_seed() {
        let tmp = TempDir::new().unwrap();
        let skills_path = tmp.path().join("skills.md");
        let seed = "# Immutable Header\nDo not change\n<!-- AGENT APPENDABLE SECTION -->\n";
        std::fs::write(&skills_path, seed).unwrap();

        append_to_skills(tmp.path(), "New knowledge").unwrap();

        let content = std::fs::read_to_string(&skills_path).unwrap();
        assert!(content.starts_with("# Immutable Header\nDo not change"));
    }

    #[test]
    fn test_read_nonexistent_skills() {
        let tmp = TempDir::new().unwrap();
        assert!(read_skills(tmp.path()).is_err());
    }
}
