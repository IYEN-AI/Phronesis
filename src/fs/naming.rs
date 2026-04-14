use regex::Regex;
use std::sync::LazyLock;

use crate::types::Warning;

// Pattern: verb_object[_method].jsonl
// At minimum: two underscore-separated segments ending in .jsonl
static NAMING_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-z][a-z0-9]*(_[a-z][a-z0-9]*)+\.jsonl$").unwrap());

/// Validate a filename against the naming convention (verb_object[_method].jsonl).
/// Returns None if valid, Some(Warning) if violated.
pub fn validate_name(filename: &str, file_path: &str) -> Option<Warning> {
    if NAMING_PATTERN.is_match(filename) {
        None
    } else {
        Some(Warning {
            ts: chrono::Utc::now().to_rfc3339(),
            file_path: file_path.to_string(),
            message: format!(
                "Filename '{}' does not match naming convention: verb_object[_method].jsonl",
                filename
            ),
            rule_violated: "naming_convention".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_names() {
        assert!(validate_name("send_email_politely.jsonl", "/test").is_none());
        assert!(validate_name("analyze_data_patterns.jsonl", "/test").is_none());
        assert!(validate_name("greet_user.jsonl", "/test").is_none());
        assert!(validate_name("handle_error_gracefully.jsonl", "/test").is_none());
    }

    #[test]
    fn test_invalid_names() {
        // Missing verb_object pattern
        assert!(validate_name("single.jsonl", "/test").is_some());
        // Wrong extension
        assert!(validate_name("send_email.txt", "/test").is_some());
        // Spaces
        assert!(validate_name("bad name.jsonl", "/test").is_some());
        // Uppercase
        assert!(validate_name("Send_Email.jsonl", "/test").is_some());
        // No extension
        assert!(validate_name("send_email", "/test").is_some());
        // Empty
        assert!(validate_name("", "/test").is_some());
    }

    #[test]
    fn test_unicode_names_rejected() {
        assert!(validate_name("보내다_이메일.jsonl", "/test").is_some());
    }

    #[test]
    fn test_starts_with_digit() {
        assert!(validate_name("1send_email.jsonl", "/test").is_some());
        assert!(validate_name("0_action.jsonl", "/test").is_some());
    }

    #[test]
    fn test_consecutive_underscores() {
        assert!(validate_name("send__email.jsonl", "/test").is_some());
    }

    #[test]
    fn test_trailing_underscore() {
        assert!(validate_name("send_email_.jsonl", "/test").is_some());
    }

    #[test]
    fn test_very_long_name() {
        let long_name = format!("{}_{}.jsonl", "a".repeat(100), "b".repeat(100));
        assert!(
            validate_name(&long_name, "/test").is_none(),
            "Long but valid name should pass"
        );
    }

    #[test]
    fn test_minimal_valid_name() {
        assert!(validate_name("a_b.jsonl", "/test").is_none());
    }

    #[test]
    fn test_numbers_in_segments() {
        assert!(validate_name("handle_error42.jsonl", "/test").is_none());
        assert!(validate_name("retry3_connection.jsonl", "/test").is_none());
    }

    #[test]
    fn test_only_extension() {
        assert!(validate_name(".jsonl", "/test").is_some());
    }

    #[test]
    fn test_warning_fields_populated() {
        let warning = validate_name("BAD", "/some/path").unwrap();
        assert_eq!(warning.file_path, "/some/path");
        assert_eq!(warning.rule_violated, "naming_convention");
        assert!(!warning.message.is_empty());
        assert!(!warning.ts.is_empty());
    }
}
