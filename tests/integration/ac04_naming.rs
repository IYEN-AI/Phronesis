use phronesis::fs::naming;

/// AC-4: Generated files follow naming convention (warning on violation)
#[test]
fn ac04_naming_validation() {
    // Valid name: no warning
    assert!(naming::validate_name("analyze_data_patterns.jsonl", "/test").is_none());
    assert!(naming::validate_name("send_email_politely.jsonl", "/test").is_none());
    assert!(naming::validate_name("handle_error_gracefully.jsonl", "/test").is_none());

    // Invalid name: warning returned
    let warning = naming::validate_name("bad name.jsonl", "/test");
    assert!(warning.is_some());
    assert_eq!(warning.unwrap().rule_violated, "naming_convention");

    // Invalid: single word
    assert!(naming::validate_name("single.jsonl", "/test").is_some());

    // Invalid: wrong extension
    assert!(naming::validate_name("send_email.txt", "/test").is_some());

    // Invalid: uppercase
    assert!(naming::validate_name("Send_Email.jsonl", "/test").is_some());
}
