use std::io::{BufRead, BufReader};
use std::path::Path;

use regex::Regex;
use walkdir::WalkDir;

use crate::error::Result;
use crate::types::{ActionFile, MatchedLine};

/// Search for files matching a regex pattern in filenames and content.
/// Returns at most `max_results` matches (default 50).
pub fn grep_search(
    folder: &Path,
    pattern: &str,
    max_results: Option<usize>,
) -> Result<Vec<ActionFile>> {
    let max = max_results.unwrap_or(50);
    let regex = Regex::new(pattern)
        .map_err(|e| crate::error::PhronesisError::Validation(format!("Invalid regex: {}", e)))?;

    let mut results = Vec::new();

    for entry in WalkDir::new(folder)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        if results.len() >= max {
            break;
        }

        let path = entry.path();
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        // Skip hidden files and meta files
        if filename.starts_with('.') {
            continue;
        }

        let mut matched_lines = Vec::new();

        // Check filename match
        if regex.is_match(filename) {
            matched_lines.push(MatchedLine {
                line_number: 0,
                content: filename.to_string(),
                is_filename_match: true,
            });
        }

        // Check content match (only for .jsonl files)
        if filename.ends_with(".jsonl") {
            if let Ok(file) = std::fs::File::open(path) {
                let reader = BufReader::new(file);
                for (line_num, line) in reader.lines().enumerate() {
                    if let Ok(line) = line {
                        if regex.is_match(&line) {
                            matched_lines.push(MatchedLine {
                                line_number: line_num + 1,
                                content: line,
                                is_filename_match: false,
                            });
                        }
                    }
                }
            }
        }

        if !matched_lines.is_empty() {
            let rel_path = path
                .strip_prefix(folder)
                .unwrap_or(path)
                .to_string_lossy()
                .to_string();
            results.push(ActionFile {
                path: rel_path,
                matched_lines,
            });
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_grep_filename_match() {
        let tmp = TempDir::new().unwrap();
        let file_path = tmp.path().join("send_email_politely.jsonl");
        std::fs::write(&file_path, "{\"ts\":\"t1\",\"action\":\"send\"}\n").unwrap();

        let results = grep_search(tmp.path(), "send_email", None).unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].matched_lines.iter().any(|m| m.is_filename_match));
    }

    #[test]
    fn test_grep_content_match() {
        let tmp = TempDir::new().unwrap();
        let file_path = tmp.path().join("handle_error.jsonl");
        std::fs::write(
            &file_path,
            "{\"ts\":\"t1\",\"action\":\"retry connection\"}\n{\"ts\":\"t2\",\"action\":\"log error\"}\n",
        )
        .unwrap();

        let results = grep_search(tmp.path(), "retry", None).unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0]
            .matched_lines
            .iter()
            .any(|m| !m.is_filename_match && m.content.contains("retry")));
    }

    #[test]
    fn test_grep_max_results() {
        let tmp = TempDir::new().unwrap();
        for i in 0..10 {
            let file_path = tmp.path().join(format!("action_{}.jsonl", i));
            std::fs::write(&file_path, "{\"ts\":\"t1\"}\n").unwrap();
        }

        let results = grep_search(tmp.path(), "action", Some(3)).unwrap();
        assert!(results.len() <= 3);
    }

    #[test]
    fn test_grep_no_match() {
        let tmp = TempDir::new().unwrap();
        let file_path = tmp.path().join("greet_user.jsonl");
        std::fs::write(&file_path, "{\"ts\":\"t1\"}\n").unwrap();

        let results = grep_search(tmp.path(), "nonexistent_pattern", None).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_grep_nested_dirs() {
        let tmp = TempDir::new().unwrap();
        let nested = tmp.path().join("sub/deep");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(
            nested.join("handle_deep_task.jsonl"),
            "{\"ts\":\"t1\"}\n",
        )
        .unwrap();

        let results = grep_search(tmp.path(), "deep_task", None).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_grep_skips_hidden_files() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join(".meta.jsonl"), "{\"description\":\"test\"}\n").unwrap();
        std::fs::write(tmp.path().join("visible.jsonl"), "{\"ts\":\"t1\"}\n").unwrap();

        let results = grep_search(tmp.path(), "test", None).unwrap();
        // Should not match .meta.jsonl
        assert!(results.iter().all(|r| !r.path.contains(".meta")));
    }
}
