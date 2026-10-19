//! A point's identity is the coordinate, not the observation: it excludes the
//! verdict and names no engine.

use serde_json::Value;
use std::collections::BTreeMap;
use std::io::Write;
use std::process::Command;

const SAMPLE: &str = "tests/fixtures/common/sample.xif.yaml";

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

fn point_ids(payload: &Value) -> BTreeMap<Vec<i64>, String> {
    payload["results"]
        .as_array()
        .expect("results")
        .iter()
        .map(|result| {
            let values: Vec<i64> = result["combination"]
                .as_array()
                .expect("combination")
                .iter()
                .map(|value| value.as_i64().expect("field value"))
                .collect();
            let point_id = result["point_id"].as_str().expect("point_id").to_string();
            (values, point_id)
        })
        .collect()
}

fn write_spec(yaml: &str) -> tempfile::NamedTempFile {
    let mut file = tempfile::NamedTempFile::new().expect("temp file");
    file.write_all(yaml.as_bytes()).expect("write spec");
    file
}

#[test]
fn point_ids_are_stable_across_runs() {
    let first = point_ids(&payload(&run(
        &["verify", "--target", SAMPLE, "--json"],
        &[],
    )));
    let second = point_ids(&payload(&run(
        &["verify", "--target", SAMPLE, "--json"],
        &[],
    )));

    assert!(!first.is_empty(), "the fixture emits points");
    assert_eq!(first, second);
}

#[test]
fn point_ids_ignore_comment_edits() {
    let plain = write_spec("target: t\nfields:\n  a:\n    range: [0, 3]\n");
    let commented =
        write_spec("# header\ntarget: t\n\nfields:\n  # field note\n  a:\n    range: [0, 3]\n");

    let first = point_ids(&payload(&run(
        &[
            "verify",
            "--target",
            plain.path().to_str().expect("path"),
            "--json",
        ],
        &[],
    )));
    let second = point_ids(&payload(&run(
        &[
            "verify",
            "--target",
            commented.path().to_str().expect("path"),
            "--json",
        ],
        &[],
    )));

    assert_eq!(
        first, second,
        "comment edits must not move a point's identity"
    );
}

#[test]
fn point_ids_are_independent_of_the_engine() {
    let structural = point_ids(&payload(&run(
        &["verify", "--target", SAMPLE, "--json"],
        &[("EV_CLASSIFIER", "structural")],
    )));
    let naive = point_ids(&payload(&run(
        &["verify", "--target", SAMPLE, "--json"],
        &[("EV_CLASSIFIER", "naive")],
    )));

    assert_eq!(
        structural, naive,
        "a point's identity must not name the engine"
    );
}
