use std::process::Command;

#[test]
fn check_help_succeeds() {
    let output = Command::new(env!("CARGO_BIN_EXE_ev"))
        .arg("check")
        .arg("--help")
        .output()
        .expect("failed to run ev check --help");
    assert!(output.status.success(), "ev check --help should exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--target"), "help should mention --target");
}

#[test]
fn certify_help_succeeds() {
    let output = Command::new(env!("CARGO_BIN_EXE_ev"))
        .arg("certify")
        .arg("--help")
        .output()
        .expect("failed to run ev certify --help");
    assert!(output.status.success(), "ev certify --help should exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--target"), "help should mention --target");
}

#[test]
fn check_json_flag_produces_valid_output() {
    let output = Command::new(env!("CARGO_BIN_EXE_ev"))
        .arg("check")
        .arg("--target")
        .arg("tests/fixtures/sample.xif.yaml")
        .arg("--json")
        .output()
        .expect("failed to run ev check --json");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("\"passed\": 12"),
        "json output should report 12 passed (eq constraint filters to 12)"
    );
    assert!(
        stdout.contains("\"field_order\""),
        "json output should include field_order"
    );
    assert!(
        stdout.contains("\"failed\": 84"),
        "json output should report 84 failed"
    );
}

#[test]
fn version_flag_succeeds() {
    let output = Command::new(env!("CARGO_BIN_EXE_ev"))
        .arg("--version")
        .output()
        .expect("failed to run ev --version");
    assert!(output.status.success(), "ev --version should exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("ev"), "version output should contain ev");
}
