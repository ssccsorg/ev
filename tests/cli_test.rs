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
fn verify_ibex_csr_access_fixture() {
    let output = Command::new(env!("CARGO_BIN_EXE_ev"))
        .arg("verify")
        .arg("--target")
        .arg("tests/fixtures/ibex/csr_access.xif.yaml")
        .arg("--json")
        .output()
        .expect("failed to run ev verify on the ibex_csr_access fixture");
    assert!(
        output.status.success(),
        "ibex_csr_access fixture should pass"
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
fn verify_enable_mask_demo_fixture() {
    let output = Command::new(env!("CARGO_BIN_EXE_ev"))
        .arg("verify")
        .arg("--target")
        .arg("tests/fixtures/common/enable_mask_demo.xif.yaml")
        .arg("--json")
        .output()
        .expect("failed to run ev verify on the enable_mask_demo fixture");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("fact_type"),
        "enable_mask_demo should produce fact output"
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
fn verify_tagma_decoder_domain_fixture() {
    let output = Command::new(env!("CARGO_BIN_EXE_ev"))
        .arg("verify")
        .arg("--target")
        .arg("tests/fixtures/tagma/tagma_decoder.xif.yaml")
        .output()
        .expect("failed to run ev verify on tagma_decoder fixture");
    // The decoder domain has 54,364 invalid code points, so the exit code is non-zero.
    assert!(
        !output.status.success(),
        "tagma_decoder fixture should exit non-zero"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("passed: 11172") && stdout.contains("failed: 54364"),
        "should report the 11,172 valid syllables: {}",
        stdout
    );
}

#[test]
fn verify_tagma_demo_top_fixture() {
    let output = Command::new(env!("CARGO_BIN_EXE_ev"))
        .arg("verify")
        .arg("--target")
        .arg("tests/fixtures/tagma/tagma_demo_top.xif.yaml")
        .output()
        .expect("failed to run ev verify on tagma_demo_top fixture");
    assert!(
        output.status.success(),
        "tagma_demo_top fixture should pass"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("All combinations passed"),
        "demo top output space should pass entirely: {}",
        stdout
    );
}

#[test]
fn verify_cva6_xif_ref_r4_fixture() {
    let output = Command::new(env!("CARGO_BIN_EXE_ev"))
        .arg("verify")
        .arg("--target")
        .arg("tests/fixtures/cva6/xif_ref_r4.xif.yaml")
        .output()
        .expect("failed to run ev verify on cva6_xif_ref_r4 fixture");
    // The fixture has invalid encodings, so the exit code is non-zero.
    assert!(
        !output.status.success(),
        "cva6_xif_ref_r4 should exit non-zero"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("total:  16384")
            && stdout.contains("passed: 2560")
            && stdout.contains("failed: 13824"),
        "should report the R4 encoding counts: {}",
        stdout
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
fn synth_tagma_decoder_with_mock_backend() {
    let output = Command::new(env!("CARGO_BIN_EXE_ev"))
        .arg("synth")
        .arg("--target")
        .arg("tests/fixtures/tagma/tagma_decoder.xif.yaml")
        .env("EV_SYNTH_BACKEND", "mock")
        .output()
        .expect("failed to run ev synth on tagma fixture");
    assert!(
        output.status.success(),
        "ev synth should exit 0 on the tagma fixture"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("[ok]"), "synthesis should show ok status");
}

#[test]
fn synth_design_uses_the_file_stem_as_top() {
    let output = Command::new(env!("CARGO_BIN_EXE_ev"))
        .arg("synth")
        .arg("--design")
        .arg("tests/fixtures/rtl/decode_demo.v")
        .env("EV_SYNTH_BACKEND", "mock")
        .output()
        .expect("failed to run ev synth --design");
    assert!(output.status.success(), "ev synth --design should exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Synthesis: decode_demo [ok]"),
        "--design without --top should resolve the file stem: {}",
        stdout
    );
}

#[test]
fn synth_design_honours_the_top_override() {
    let output = Command::new(env!("CARGO_BIN_EXE_ev"))
        .arg("synth")
        .arg("--design")
        .arg("tests/fixtures/rtl/decode_demo.v")
        .arg("--top")
        .arg("decode_demo_helper")
        .env("EV_SYNTH_BACKEND", "mock")
        .output()
        .expect("failed to run ev synth --design --top");
    assert!(
        output.status.success(),
        "ev synth --design --top should exit 0"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Synthesis: decode_demo_helper [ok]"),
        "--top should select the module: {}",
        stdout
    );
}

#[test]
fn synth_design_json_names_the_design_source() {
    let output = Command::new(env!("CARGO_BIN_EXE_ev"))
        .arg("synth")
        .arg("--design")
        .arg("tests/fixtures/rtl/decode_demo.v")
        .arg("--json")
        .env("EV_SYNTH_BACKEND", "mock")
        .output()
        .expect("failed to run ev synth --design --json");
    assert!(
        output.status.success(),
        "ev synth --design --json should exit 0"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let fact: serde_json::Value =
        serde_json::from_str(&stdout).expect("ev synth --json should emit a Fact envelope");
    assert_eq!(fact["fact_type"], "synthesis_result", "fact type");
    assert_eq!(
        fact["target"], "decode_demo",
        "the fact target is the resolved top module"
    );

    // The payload is an opaque byte vector in the envelope, so decode it the
    // way a consumer would and check that it names the design file rather
    // than RTL generated from a spec.
    let payload_bytes = fact["payload"]
        .as_array()
        .expect("the payload should be a byte array")
        .iter()
        .map(|byte| byte.as_u64().expect("payload bytes are numbers") as u8)
        .collect::<Vec<u8>>();
    let payload: serde_json::Value =
        serde_json::from_slice(&payload_bytes).expect("the payload should be JSON");
    let source = payload["source"].as_str().expect("payload source");
    assert!(
        source.contains("decode_demo.v") && !source.contains("tmp"),
        "the source should be the design file: {source}"
    );
}

#[test]
fn synth_design_missing_file_fails() {
    let output = Command::new(env!("CARGO_BIN_EXE_ev"))
        .arg("synth")
        .arg("--design")
        .arg("tests/fixtures/rtl/absent.v")
        .env("EV_SYNTH_BACKEND", "mock")
        .output()
        .expect("failed to run ev synth --design on a missing file");
    assert!(
        !output.status.success(),
        "a missing design file should fail"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("design file not found"),
        "the error should name the cause: {}",
        stderr
    );
}

#[test]
fn synth_requires_target_or_design() {
    let output = Command::new(env!("CARGO_BIN_EXE_ev"))
        .arg("synth")
        .output()
        .expect("failed to run bare ev synth");
    assert!(
        !output.status.success(),
        "ev synth without an input should fail"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--design"),
        "the error should name the required argument: {}",
        stderr
    );
}

#[test]
fn synth_rejects_target_with_top() {
    let output = Command::new(env!("CARGO_BIN_EXE_ev"))
        .arg("synth")
        .arg("--target")
        .arg("tests/fixtures/common/all_pass.xif.yaml")
        .arg("--top")
        .arg("decode_demo")
        .output()
        .expect("failed to run ev synth --target --top");
    assert!(
        !output.status.success(),
        "--top is a --design-only option and should be rejected"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("cannot be used with"),
        "the error should explain the conflict: {}",
        stderr
    );
}

#[test]
fn synth_rejects_target_with_design() {
    let output = Command::new(env!("CARGO_BIN_EXE_ev"))
        .arg("synth")
        .arg("--target")
        .arg("tests/fixtures/common/all_pass.xif.yaml")
        .arg("--design")
        .arg("tests/fixtures/rtl/decode_demo.v")
        .output()
        .expect("failed to run ev synth --target --design");
    assert!(
        !output.status.success(),
        "--target and --design are mutually exclusive"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("cannot be used with"),
        "the error should explain the conflict: {}",
        stderr
    );
}

#[test]
fn synth_design_rejects_an_argument_yosys_cannot_carry() {
    let output = Command::new(env!("CARGO_BIN_EXE_ev"))
        .arg("synth")
        .arg("--design")
        .arg("tests/fixtures/rtl/decode demo.v")
        .env("EV_SYNTH_BACKEND", "mock")
        .output()
        .expect("failed to run ev synth --design with whitespace in the path");
    assert!(
        !output.status.success(),
        "a design path with whitespace should fail"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("must not contain whitespace"),
        "the error should name the restriction: {}",
        stderr
    );

    let output = Command::new(env!("CARGO_BIN_EXE_ev"))
        .arg("synth")
        .arg("--design")
        .arg("tests/fixtures/rtl/decode_demo.v")
        .arg("--top")
        .arg("decode;demo")
        .env("EV_SYNTH_BACKEND", "mock")
        .output()
        .expect("failed to run ev synth --top with a command separator");
    assert!(!output.status.success(), "a top name with ';' should fail");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("must not contain whitespace"),
        "the error should name the restriction: {}",
        stderr
    );
}

#[test]
fn verify_cva6_xif_ref_fixture() {
    let output = Command::new(env!("CARGO_BIN_EXE_ev"))
        .arg("verify")
        .arg("--target")
        .arg("tests/fixtures/cva6/xif_ref.xif.yaml")
        .output()
        .expect("failed to run ev verify on cva6_xif_ref fixture");
    // Most of the 33.5M space is invalid, so the exit code is non-zero.
    assert!(
        !output.status.success(),
        "cva6_xif_ref should exit non-zero"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("total:  33554432")
            && stdout.contains("passed: 196608")
            && stdout.contains("failed: 33357824"),
        "should report the full CVA6 decoder counts: {}",
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
