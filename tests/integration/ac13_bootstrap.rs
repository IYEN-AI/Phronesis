use phronesis::bootstrap;
use tempfile::TempDir;

/// AC-13: 6-Pillar + skills.md auto-creation on bootstrap
#[test]
fn ac13_bootstrap_creates_full_structure() {
    let tmp = TempDir::new().unwrap();
    bootstrap::bootstrap(tmp.path()).unwrap();

    // All 6 pillar directories exist
    for pillar in ["self", "perception", "cognition", "praxis", "evolution", "reflection"] {
        let dir = tmp.path().join(pillar);
        assert!(dir.is_dir(), "Missing pillar directory: {}", pillar);

        // Each has .meta.jsonl
        let meta = dir.join(".meta.jsonl");
        assert!(meta.exists(), "Missing .meta.jsonl in {}", pillar);

        // Meta contains a description
        let content = std::fs::read_to_string(&meta).unwrap();
        assert!(!content.trim().is_empty(), "Empty .meta.jsonl in {}", pillar);
    }

    // skills.md exists with seed content
    let skills = tmp.path().join("skills.md");
    assert!(skills.exists());
    let skills_content = std::fs::read_to_string(&skills).unwrap();
    assert!(skills_content.contains("6-Pillar"));
    assert!(skills_content.contains("AGENT APPENDABLE SECTION"));

    // .index directory exists
    assert!(tmp.path().join(".index").is_dir());

    // Warning log file exists
    assert!(tmp
        .path()
        .join("reflection/warnings/naming_violations.jsonl")
        .exists());
}
