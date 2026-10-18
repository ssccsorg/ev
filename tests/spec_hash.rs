//! Content-address coverage: the layout is part of the space, the YAML
//! presentation is not, and the address is stable across processes.

use ev::report::hash_spec;
use ev::spec::{FieldBitMapping, VerificationSpec, WordLayout};
use std::collections::BTreeMap;
use std::io::Write;
use std::process::Command;

fn parse(yaml: &str) -> VerificationSpec {
    let mut file = tempfile::NamedTempFile::new().expect("temp file");
    file.write_all(yaml.as_bytes()).expect("write spec");
    VerificationSpec::from_yaml(file.path()).expect("the spec parses")
}

fn layout(pos: u32, width: u32) -> WordLayout {
    WordLayout {
        word_width: Some(16),
        field_map: BTreeMap::from([("a".to_string(), FieldBitMapping { pos, width })]),
    }
}

const BASE: &str = "target: t\nfields:\n  a:\n    range: [0, 3]\n";

#[test]
fn a_layout_moves_the_address_away_from_no_layout() {
    let without = parse(BASE);
    let mut with = parse(BASE);
    with.layout = Some(layout(0, 2));
    assert_ne!(hash_spec(&without), hash_spec(&with));
}

#[test]
fn a_bit_position_moves_the_address() {
    let mut first = parse(BASE);
    first.layout = Some(layout(0, 2));
    let mut second = parse(BASE);
    second.layout = Some(layout(2, 2));
    assert_ne!(hash_spec(&first), hash_spec(&second));
}

#[test]
fn a_word_width_moves_the_address() {
    let mut first = parse(BASE);
    first.layout = Some(layout(0, 2));
    let mut second = parse(BASE);
    second.layout = Some(WordLayout {
        word_width: Some(32),
        field_map: layout(0, 2).field_map,
    });
    assert_ne!(hash_spec(&first), hash_spec(&second));
}

#[test]
fn cross_mapping_order_does_not_move_the_address() {
    let forward = parse(
        "target: t\nfields:\n  a:\n    range: [0, 1]\n  b:\n    range: [0, 2]\nconstraints:\n  - type: cross\n    field_a: a\n    field_b: b\n    mapping:\n      0: [0, 1]\n      1: [2]\n",
    );
    let reverse = parse(
        "target: t\nfields:\n  a:\n    range: [0, 1]\n  b:\n    range: [0, 2]\nconstraints:\n  - type: cross\n    field_a: a\n    field_b: b\n    mapping:\n      1: [2]\n      0: [0, 1]\n",
    );
    assert_eq!(hash_spec(&forward), hash_spec(&reverse));
}

#[test]
fn comments_do_not_move_the_address() {
    let plain = parse(BASE);
    let commented = parse(
        "# header\n# second line\ntarget: t\n\nfields:\n  # field note\n  a:\n    range: [0, 3]\n",
    );
    assert_eq!(hash_spec(&plain), hash_spec(&commented));
}

#[test]
fn the_address_is_stable_across_processes() {
    let first = run_spec_hash("tests/fixtures/common/enable_mask_demo.xif.yaml");
    let second = run_spec_hash("tests/fixtures/common/enable_mask_demo.xif.yaml");
    assert_eq!(
        first, second,
        "two runs must address the space the same way"
    );
}

fn run_spec_hash(fixture: &str) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_ev"))
        .args(["verify", "--target", fixture, "--json"])
        .output()
        .expect("run ev");
    assert!(!output.stdout.is_empty(), "the report is on stdout");

    let fact: serde_json::Value = serde_json::from_slice(&output.stdout).expect("Fact JSON");
    let payload = fact["payload"].as_array().expect("payload bytes");
    let bytes: Vec<u8> = payload
        .iter()
        .map(|value| value.as_u64().expect("byte") as u8)
        .collect();
    let report: serde_json::Value = serde_json::from_slice(&bytes).expect("report JSON");
    report["spec_hash"].as_str().expect("spec_hash").to_string()
}
