use phronesis::bootstrap;
use phronesis::fs::warnings;
use phronesis::types::Warning;
use tempfile::TempDir;

/// AC-10: get_warnings returns accumulated warning log, filterable by since
#[test]
fn ac10_warning_query_with_since() {
    let tmp = TempDir::new().unwrap();
    bootstrap::bootstrap(tmp.path()).unwrap();

    let w1 = Warning {
        ts: "2026-04-14T10:00:00Z".into(),
        file_path: "/praxis/bad1.txt".into(),
        message: "Bad name 1".into(),
        rule_violated: "naming_convention".into(),
    };
    let w2 = Warning {
        ts: "2026-04-14T12:00:00Z".into(),
        file_path: "/praxis/bad2.txt".into(),
        message: "Bad name 2".into(),
        rule_violated: "naming_convention".into(),
    };
    let w3 = Warning {
        ts: "2026-04-14T14:00:00Z".into(),
        file_path: "/praxis/bad3.txt".into(),
        message: "Bad name 3".into(),
        rule_violated: "naming_convention".into(),
    };

    warnings::log_warning(tmp.path(), &w1).unwrap();
    warnings::log_warning(tmp.path(), &w2).unwrap();
    warnings::log_warning(tmp.path(), &w3).unwrap();

    // All warnings
    let all = warnings::get_warnings(tmp.path(), None).unwrap();
    assert_eq!(all.len(), 3);

    // Since filter: only after 11:00
    let filtered = warnings::get_warnings(tmp.path(), Some("2026-04-14T11:00:00Z")).unwrap();
    assert_eq!(filtered.len(), 2);
    assert_eq!(filtered[0].file_path, "/praxis/bad2.txt");
    assert_eq!(filtered[1].file_path, "/praxis/bad3.txt");

    // Since filter: only after 13:00
    let filtered = warnings::get_warnings(tmp.path(), Some("2026-04-14T13:00:00Z")).unwrap();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].file_path, "/praxis/bad3.txt");
}
