use phronesis::bootstrap;
use phronesis::fs::{naming, warnings};
use tempfile::TempDir;

/// AC-9: Naming warnings accumulate in /reflection warning log
#[test]
fn ac09_warnings_accumulate() {
    let tmp = TempDir::new().unwrap();
    bootstrap::bootstrap(tmp.path()).unwrap();

    // Generate 3 naming violations
    for name in ["BAD.txt", "also bad.jsonl", "x.jsonl"] {
        if let Some(warning) = naming::validate_name(name, &format!("/praxis/{}", name)) {
            warnings::log_warning(tmp.path(), &warning).unwrap();
        }
    }

    // Read the warning log file directly
    let log_path = tmp
        .path()
        .join("reflection/warnings/naming_violations.jsonl");
    let content = std::fs::read_to_string(&log_path).unwrap();
    let line_count = content.lines().filter(|l| !l.trim().is_empty()).count();
    assert_eq!(line_count, 3);

    // Also verify via get_warnings
    let all_warnings = warnings::get_warnings(tmp.path(), None).unwrap();
    assert_eq!(all_warnings.len(), 3);
}
