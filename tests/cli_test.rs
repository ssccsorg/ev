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
fn check_json_flag_succeeds() {
    let output = Command::new(env!("CARGO_BIN_EXE_ev"))
        .arg("check")
        .arg("--target")
        .arg("tests/fixtures/sample.xif.yaml")
        .arg("--json")
        .output()
        .expect("failed to run ev check --json");
    assert!(output.status.success(), "ev check --json should exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("\"passed\": 96"),
        "json output should report 96 passed"
    );
    assert!(
        stdout.contains("\"field_order\""),
        "json output should include field_order"
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
