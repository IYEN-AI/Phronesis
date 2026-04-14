use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

use crate::error::Result;
use crate::types::Warning;

/// Log a warning to the naming violations log.
pub fn log_warning(data_root: &Path, warning: &Warning) -> Result<()> {
    let warnings_path = data_root.join("reflection/warnings/naming_violations.jsonl");
    if let Some(parent) = warnings_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let line = serde_json::to_string(warning)?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&warnings_path)?;
    writeln!(file, "{}", line)?;
    Ok(())
}

/// Get warnings, optionally filtered by a "since" timestamp.
/// Skips unparseable lines with a tracing warning (defensive parsing).
pub fn get_warnings(data_root: &Path, since: Option<&str>) -> Result<Vec<Warning>> {
    let warnings_path = data_root.join("reflection/warnings/naming_violations.jsonl");
    if !warnings_path.exists() {
        return Ok(Vec::new());
    }

    let file = std::fs::File::open(&warnings_path)?;
    let reader = BufReader::new(file);
    let mut warnings = Vec::new();

    for (line_num, line) in reader.lines().enumerate() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match serde_json::from_str::<Warning>(trimmed) {
            Ok(warning) => {
                if let Some(since_ts) = since {
                    if warning.ts.as_str() > since_ts {
                        warnings.push(warning);
                    }
                } else {
                    warnings.push(warning);
                }
            }
            Err(err) => {
                tracing::warn!(
                    "Skipping corrupt JSONL line {} in {}: {}",
                    line_num + 1,
                    warnings_path.display(),
                    err
                );
            }
        }
    }

    Ok(warnings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_log_and_get_warnings() {
        let tmp = TempDir::new().unwrap();
        let w1 = Warning {
            ts: "2026-04-14T10:00:00Z".into(),
            file_path: "/praxis/bad.txt".into(),
            message: "Bad name".into(),
            rule_violated: "naming_convention".into(),
        };
        let w2 = Warning {
            ts: "2026-04-14T11:00:00Z".into(),
            file_path: "/praxis/also_bad.txt".into(),
            message: "Also bad".into(),
            rule_violated: "naming_convention".into(),
        };

        log_warning(tmp.path(), &w1).unwrap();
        log_warning(tmp.path(), &w2).unwrap();

        let all = get_warnings(tmp.path(), None).unwrap();
        assert_eq!(all.len(), 2);

        let filtered = get_warnings(tmp.path(), Some("2026-04-14T10:30:00Z")).unwrap();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].file_path, "/praxis/also_bad.txt");
    }

    #[test]
    fn test_get_warnings_empty() {
        let tmp = TempDir::new().unwrap();
        let warnings = get_warnings(tmp.path(), None).unwrap();
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_defensive_parsing() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("reflection/warnings/naming_violations.jsonl");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            "{\"ts\":\"t1\",\"file_path\":\"f\",\"message\":\"m\",\"rule_violated\":\"r\"}\n{corrupt\n",
        )
        .unwrap();

        let warnings = get_warnings(tmp.path(), None).unwrap();
        assert_eq!(warnings.len(), 1);
    }
}
