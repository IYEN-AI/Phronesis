use phronesis::bootstrap;
use phronesis::fs::skills;
use tempfile::TempDir;

/// AC-14: Agent can append to skills.md without modifying seed section
#[test]
fn ac14_skills_append() {
    let tmp = TempDir::new().unwrap();
    bootstrap::bootstrap(tmp.path()).unwrap();

    // Read original seed
    let original = skills::read_skills(tmp.path()).unwrap();
    assert!(original.contains("6-Pillar"));

    // Agent appends learned pattern
    skills::append_to_skills(tmp.path(), "## My Learned Pattern\n\nI discovered that greeting users warmly leads to better outcomes.").unwrap();

    // Read updated skills
    let updated = skills::read_skills(tmp.path()).unwrap();

    // Seed section intact
    assert!(updated.contains("6-Pillar"));
    assert!(updated.contains("네이밍 규칙"));
    assert!(updated.contains("AGENT APPENDABLE SECTION"));

    // Appended content present
    assert!(updated.contains("My Learned Pattern"));
    assert!(updated.contains("greeting users warmly"));
}
