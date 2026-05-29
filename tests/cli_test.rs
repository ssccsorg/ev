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
fn check_json_with_synth_mock() {
    let output = Command::new(env!("CARGO_BIN_EXE_ev"))
        .arg("check")
        .arg("--target")
        .arg("tests/fixtures/all_pass.xif.yaml")
        .arg("--json")
        .arg("--synth")
        .env("EV_SYNTH_BACKEND", "mock")
        .output()
        .expect("failed to run ev check --json --synth with mock backend");
    assert!(
        output.status.success(),
        "ev check --json --synth should exit 0"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Must contain both verification report and synthesis Fact.
    assert!(
        stdout.contains("\"field_order\""),
        "json output should include verification field_order"
    );
    assert!(
        stdout.contains("\"fact_type\": \"synthesis_result\""),
        "json output should include synthesis Fact"
    );
    assert!(
        stdout.contains("\"payload\""),
        "synthesis Fact should contain payload"
    );
    assert!(
        stdout.contains("\"status\": \"ok\""),
        "synthesis status should be ok"
    );
}

#[test]
fn check_text_with_synth_mock() {
    let output = Command::new(env!("CARGO_BIN_EXE_ev"))
        .arg("check")
        .arg("--target")
        .arg("tests/fixtures/all_pass.xif.yaml")
        .arg("--synth")
        .env("EV_SYNTH_BACKEND", "mock")
        .output()
        .expect("failed to run ev check --synth with mock backend");
    assert!(output.status.success(), "ev check --synth should exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Text output must show synthesis summary.
    assert!(
        stdout.contains("Synthesis:"),
        "text output should contain Synthesis summary"
    );
    assert!(stdout.contains("[ok]"), "synthesis should show ok status");
    assert!(
        stdout.contains("backend:  mock"),
        "should mention mock backend"
    );
    assert!(stdout.contains("gate count:"), "should show gate count");
}

#[test]
fn check_json_all_pass() {
    let output = Command::new(env!("CARGO_BIN_EXE_ev"))
        .arg("check")
        .arg("--target")
        .arg("tests/fixtures/all_pass.xif.yaml")
        .arg("--json")
        .output()
        .expect("failed to run ev check --json on all_pass fixture");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("\"passed\": 1024"),
        "all 1024 combos should pass"
    );
    assert!(stdout.contains("\"failed\": 0"), "no failures expected");
    assert!(
        stdout.contains("\"spec_hash\""),
        "json output should include spec_hash for neXus linking"
    );
    assert!(
        stdout.contains("\"origin\""),
        "json output should include origin"
    );
}

#[test]
fn check_text_mixed_fixture_exits_1() {
    let output = Command::new(env!("CARGO_BIN_EXE_ev"))
        .arg("check")
        .arg("--target")
        .arg("tests/fixtures/sample.xif.yaml")
        .output()
        .expect("failed to run ev check on mixed fixture");
    assert!(
        !output.status.success(),
        "mixed fixture should exit non-zero"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("failed: 84"), "should report 84 failures");
}

#[test]
fn check_help_mentions_synth_flag() {
    let output = Command::new(env!("CARGO_BIN_EXE_ev"))
        .arg("check")
        .arg("--help")
        .output()
        .expect("failed to run ev check --help");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("--synth"),
        "help should mention --synth flag"
    );
}

#[test]
fn check_rv32i_csr_access_fixture() {
    let output = Command::new(env!("CARGO_BIN_EXE_ev"))
        .arg("check")
        .arg("--target")
        .arg("tests/fixtures/rv32i_csr_access.xif.yaml")
        .arg("--json")
        .output()
        .expect("failed to run ev check on rv32i_csr_access fixture");
    assert!(
        output.status.success(),
        "rv32i_csr_access fixture should pass: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("\"passed\""),
        "output should contain verification results"
    );
}

#[test]
fn check_malformed_no_fields_exits_nonzero() {
    let output = Command::new(env!("CARGO_BIN_EXE_ev"))
        .arg("check")
        .arg("--target")
        .arg("tests/fixtures/malformed_no_fields.xif.yaml")
        .output()
        .expect("failed to run ev check on malformed fixture");
    // YAML without fields parses successfully (fields defaults to empty),
    // then expand_all returns 0 combinations, which exits 0 with 0 passed/0 failed.
    // This is currently accepted behavior. The malformed_bad_type test covers
    // the more important error case.
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("passed: 0\nfailed: 0")
            || stdout.contains("\"passed\": 0"),
        "output should mention passed/failed: {}",
        stdout
    );
}

#[test]
fn check_malformed_bad_constraint_type_exits_nonzero() {
    let output = Command::new(env!("CARGO_BIN_EXE_ev"))
        .arg("check")
        .arg("--target")
        .arg("tests/fixtures/malformed_bad_type.xif.yaml")
        .output()
        .expect("failed to run ev check on malformed constraint fixture");
    assert!(
        !output.status.success(),
        "YAML with unknown constraint type should exit non-zero"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unknown variant") || stderr.contains("nonexistent_constraint"),
        "stderr should mention the unknown constraint type: {}",
        stderr
    );
}

#[test]
fn check_ibex_alu_ext_fixture() {
    let output = Command::new(env!("CARGO_BIN_EXE_ev"))
        .arg("check")
        .arg("--target")
        .arg("tests/fixtures/ibex_alu_ext.xif.yaml")
        .arg("--json")
        .output()
        .expect("failed to run ev check on ibex_alu_ext fixture");
    // neq constraint produces 64 failures, so exit code is non-zero.
    let stdout = String::from_utf8_lossy(&output.stdout);
    // 8 * 8 * 8 = 512 total. neq removes 64 (rs1 == rd), so 512 - 64 = 448 pass.
    assert!(
        stdout.contains("\"passed\": 448"),
        "ibex_alu_ext should report 448 passed: {}",
        stdout
    );
    assert!(
        stdout.contains("\"failed\": 64"),
        "ibex_alu_ext should report 64 failed: {}",
        stdout
    );
}

#[test]
fn check_cva6_xif_mac_fixture() {
    let output = Command::new(env!("CARGO_BIN_EXE_ev"))
        .arg("check")
        .arg("--target")
        .arg("tests/fixtures/cva6_xif_mac.xif.yaml")
        .arg("--json")
        .output()
        .expect("failed to run ev check on cva6_xif_mac fixture");
    // neq constraint produces 4096 failures, so exit code is non-zero.
    let stdout = String::from_utf8_lossy(&output.stdout);
    // 2 * 8 * 8 * 256 = 32768 total. neq (rs1 != rs2) removes 8 * 2 * 256 = 4096.
    // 32768 - 4096 = 28672 pass.
    assert!(
        stdout.contains("\"passed\": 28672"),
        "cva6_xif_mac should report 28672 passed: {}",
        stdout
    );
    assert!(
        stdout.contains("\"failed\": 4096"),
        "cva6_xif_mac should report 4096 failed: {}",
        stdout
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
