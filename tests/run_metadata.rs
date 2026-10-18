//! Run metadata stays in the Fact envelope: the verification and synthesis
//! payloads are functions of the spec, not of the run.

use ev::report::hash_spec;
use ev::spec::VerificationSpec;
use ev::synth::{GenerateRtl, SvGenerator};
use serde_json::Value;
use std::process::Command;

const SAMPLE: &str = "tests/fixtures/common/sample.xif.yaml";
const ALL_PASS: &str = "tests/fixtures/common/all_pass.xif.yaml";

fn run(args: &[&str], envs: &[(&str, &str)]) -> std::process::Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_ev"));
    command.args(args);
    for (key, value) in envs {
        command.env(key, value);
    }
    command.output().expect("run ev")
}

/// Decode the Fact envelope and return the JSON payload.
fn payload(output: &std::process::Output) -> Value {
    let fact: Value = serde_json::from_slice(&output.stdout).expect("Fact JSON");
    let bytes: Vec<u8> = fact["payload"]
        .as_array()
        .expect("payload bytes")
        .iter()
        .map(|value| value.as_u64().expect("byte") as u8)
        .collect();
    serde_json::from_slice(&bytes).expect("payload JSON")
}

#[test]
fn verification_payload_is_stable_across_runs() {
    let first = payload(&run(&["verify", "--target", SAMPLE, "--json"], &[]));
    let second = payload(&run(&["verify", "--target", SAMPLE, "--json"], &[]));

    assert_eq!(
        first, second,
        "two identical runs must carry identical content"
    );
    assert!(
        first.get("timestamp").is_none(),
        "the payload must not carry a run timestamp; the Fact envelope does"
    );
}

#[test]
fn synthesis_payload_is_stable_across_runs() {
    let first = payload(&run(
        &["synth", "--target", ALL_PASS, "--json"],
        &[("EV_SYNTH_BACKEND", "mock")],
    ));
    let second = payload(&run(
        &["synth", "--target", ALL_PASS, "--json"],
        &[("EV_SYNTH_BACKEND", "mock")],
    ));

    assert_eq!(
        first, second,
        "the RTL path must not move the synthesis content"
    );
}

#[test]
fn generated_rtl_path_is_a_function_of_the_spec() {
    let spec = VerificationSpec::from_yaml(std::path::Path::new(
        "tests/fixtures/common/all_pass.xif.yaml",
    ))
    .expect("parse spec");
    let first = SvGenerator.generate(&spec).expect("generate RTL");
    let second = SvGenerator.generate(&spec).expect("generate RTL");

    assert_eq!(first, second);
    let name = first.file_name().expect("file name").to_string_lossy();
    assert!(name.starts_with("simple_alu_"), "{name}");
    assert!(
        name.contains(&hash_spec(&spec)[..16]),
        "the path must carry the spec address: {name}"
    );
}

#[test]
fn concurrent_generation_leaves_complete_files() {
    let spec = VerificationSpec::from_yaml(std::path::Path::new(ALL_PASS)).expect("parse spec");
    let handles: Vec<_> = (0..8)
        .map(|_| {
            let spec = spec.clone();
            std::thread::spawn(move || {
                let path = SvGenerator.generate(&spec).expect("generate RTL");
                let content = std::fs::read_to_string(&path).expect("read RTL");
                (path, content)
            })
        })
        .collect();

    for handle in handles {
        let (path, content) = handle.join().expect("generation thread");
        assert!(
            content.contains("module simple_alu"),
            "{} is incomplete",
            path.display()
        );
    }
}
