use std::process::Command;

#[test]
fn verify_help_succeeds() {
    let output = Command::new(env!("CARGO_BIN_EXE_ev"))
        .arg("verify")
        .arg("--help")
        .output()
        .expect("failed to run ev verify --help");
    assert!(output.status.success(), "ev verify --help should exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--target"), "help should mention --target");
}

#[test]
fn verify_text_all_pass() {
    let output = Command::new(env!("CARGO_BIN_EXE_ev"))
        .arg("verify")
        .arg("--target")
        .arg("tests/fixtures/common/all_pass.xif.yaml")
        .output()
        .expect("failed to run ev verify on all_pass fixture");
    assert!(output.status.success(), "ev verify should exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("All combinations passed"),
        "all pass fixture should show all passed"
    );
}

#[test]
fn verify_json_contains_fact_envelope() {
    let output = Command::new(env!("CARGO_BIN_EXE_ev"))
        .arg("verify")
        .arg("--target")
        .arg("tests/fixtures/common/sample.xif.yaml")
        .arg("--json")
        .output()
        .expect("failed to run ev verify --json");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("fact_type"),
        "json should contain fact_type"
    );
    assert!(
        stdout.contains("payload"),
        "json should contain payload field"
    );
}

#[test]
fn verify_text_mixed_fixture_exits_1() {
    let output = Command::new(env!("CARGO_BIN_EXE_ev"))
        .arg("verify")
        .arg("--target")
        .arg("tests/fixtures/common/sample.xif.yaml")
        .output()
        .expect("failed to run ev verify on mixed fixture");
    assert!(
        !output.status.success(),
        "mixed fixture should exit non-zero"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("failed: 84"), "should report 84 failures");
}

#[test]
fn verify_rv32i_csr_access_fixture() {
    let output = Command::new(env!("CARGO_BIN_EXE_ev"))
        .arg("verify")
        .arg("--target")
        .arg("tests/fixtures/ibex/csr_access.xif.yaml")
        .arg("--json")
        .output()
        .expect("failed to run ev verify on rv32i_csr_access fixture");
    assert!(
        output.status.success(),
        "rv32i_csr_access fixture should pass"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("fact_type"),
        "output should contain fact_type"
    );
}

#[test]
fn verify_malformed_no_fields_exits_zero() {
    let output = Command::new(env!("CARGO_BIN_EXE_ev"))
        .arg("verify")
        .arg("--target")
        .arg("tests/fixtures/common/malformed_no_fields.xif.yaml")
        .output()
        .expect("failed to run ev verify on malformed fixture");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("passed: 0") && stdout.contains("failed: 0"),
        "output should mention passed/failed: {}",
        stdout
    );
}

#[test]
fn verify_malformed_bad_constraint_type_exits_nonzero() {
    let output = Command::new(env!("CARGO_BIN_EXE_ev"))
        .arg("verify")
        .arg("--target")
        .arg("tests/fixtures/common/malformed_bad_type.xif.yaml")
        .output()
        .expect("failed to run ev verify on malformed constraint fixture");
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
fn verify_ibex_alu_ext_fixture() {
    let output = Command::new(env!("CARGO_BIN_EXE_ev"))
        .arg("verify")
        .arg("--target")
        .arg("tests/fixtures/ibex/alu_ext.xif.yaml")
        .arg("--json")
        .output()
        .expect("failed to run ev verify on ibex_alu_ext fixture");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("fact_type"),
        "ibex_alu_ext should produce fact output"
    );
}

#[test]
fn verify_cva6_xif_mac_fixture() {
    let output = Command::new(env!("CARGO_BIN_EXE_ev"))
        .arg("verify")
        .arg("--target")
        .arg("tests/fixtures/cva6/xif_mac.xif.yaml")
        .arg("--json")
        .output()
        .expect("failed to run ev verify on cva6_xif_mac fixture");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("fact_type"),
        "cva6_xif_mac should produce fact output"
    );
}

#[test]
#[ignore = "medium: 2M combinations, ~25s on M1 Max (CI may timeout)"]
fn verify_cva6_xif_ref_r4_fixture() {
    let output = Command::new(env!("CARGO_BIN_EXE_ev"))
        .arg("verify")
        .arg("--target")
        .arg("tests/fixtures/cva6/xif_ref_r4.xif.yaml")
        .arg("--json")
        .output()
        .expect("failed to run ev verify on cva6_xif_ref_r4 fixture");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("fact_type"),
        "cva6_xif_ref_r4 should produce fact output"
    );
    assert!(
        stdout.contains("\"payload\""),
        "cva6_xif_ref_r4 should contain payload"
    );
}

#[test]
fn synth_text_with_mock_backend() {
    let output = Command::new(env!("CARGO_BIN_EXE_ev"))
        .arg("synth")
        .arg("--target")
        .arg("tests/fixtures/common/all_pass.xif.yaml")
        .env("EV_SYNTH_BACKEND", "mock")
        .output()
        .expect("failed to run ev synth with mock backend");
    assert!(output.status.success(), "ev synth should exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
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
fn synth_json_with_mock_backend() {
    let output = Command::new(env!("CARGO_BIN_EXE_ev"))
        .arg("synth")
        .arg("--target")
        .arg("tests/fixtures/common/all_pass.xif.yaml")
        .arg("--json")
        .env("EV_SYNTH_BACKEND", "mock")
        .output()
        .expect("failed to run ev synth --json with mock backend");
    assert!(output.status.success(), "ev synth --json should exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("fact_type"),
        "json output should include fact_type"
    );
    assert!(
        stdout.contains("payload"),
        "json output should include payload"
    );
}

#[test]
#[ignore = "slow: 33M combinations, run with -- --include-ignored (CI skips by default)"]
fn verify_cva6_xif_ref_fixture() {
    let output = Command::new(env!("CARGO_BIN_EXE_ev"))
        .arg("verify")
        .arg("--target")
        .arg("tests/fixtures/cva6/xif_ref.xif.yaml")
        .arg("--json")
        .output()
        .expect("failed to run ev verify on cva6_xif_ref fixture");
    // cva6_xif_ref has many illegal encodings → exit non-zero, which is expected.
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("fact_type"),
        "cva6_xif_ref should produce fact output"
    );
    assert!(
        stdout.contains("\"payload\""),
        "cva6_xif_ref should contain payload"
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

#[test]
fn simulate_help_succeeds() {
    let output = Command::new(env!("CARGO_BIN_EXE_ev"))
        .arg("simulate")
        .arg("--help")
        .output()
        .expect("failed to run ev simulate --help");
    assert!(output.status.success(), "ev simulate --help should exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--target"), "help should mention --target");
}

#[test]
fn synth_help_succeeds() {
    let output = Command::new(env!("CARGO_BIN_EXE_ev"))
        .arg("synth")
        .arg("--help")
        .output()
        .expect("failed to run ev synth --help");
    assert!(output.status.success(), "ev synth --help should exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--target"), "help should mention --target");
}
