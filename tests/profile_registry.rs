//! The layout abstraction: a spec either declares no word, selects a profile,
//! or carries a layout the caller registered.

use ev::profile::LayoutRegistry;
use ev::spec::{FieldBitMapping, VerificationSpec, WordLayout};
use std::collections::BTreeMap;
use std::io::Write;

fn parse_with(yaml: &str, profiles: &LayoutRegistry) -> anyhow::Result<VerificationSpec> {
    let mut file = tempfile::NamedTempFile::new().expect("temp file");
    file.write_all(yaml.as_bytes()).expect("write spec");
    VerificationSpec::from_yaml_with(file.path(), profiles)
}

fn parse(yaml: &str) -> anyhow::Result<VerificationSpec> {
    parse_with(yaml, &LayoutRegistry::default())
}

#[test]
fn a_spec_without_a_layout_has_no_word() {
    let spec = VerificationSpec::from_yaml(std::path::Path::new(
        "tests/fixtures/common/all_pass.xif.yaml",
    ))
    .expect("the generic fixture parses");
    assert!(
        spec.layout.is_none(),
        "a spec that declares no word must not inherit one"
    );
}

#[test]
fn the_riscv_profile_follows_the_feature() {
    let registry = LayoutRegistry::default();
    #[cfg(feature = "riscv")]
    {
        let layout = registry
            .resolve("riscv-r")
            .expect("the feature registers it");
        assert_eq!(layout.word_width, Some(32));
        assert_eq!(layout.field_map["opcode"].pos, 0);
    }
    #[cfg(not(feature = "riscv"))]
    assert!(
        registry.resolve("riscv-r").is_none(),
        "a build without the feature carries no RISC-V layout"
    );
}

#[test]
fn a_caller_registers_its_own_layout() {
    let mut registry = LayoutRegistry::new();
    registry.register("frame", || WordLayout {
        word_width: Some(16),
        field_map: BTreeMap::from([("kind".to_string(), FieldBitMapping { pos: 0, width: 4 })]),
    });

    let spec = parse_with(
        "target: frame_demo\nprofile: frame\nfields:\n  kind:\n    range: [0, 15]\n",
        &registry,
    )
    .expect("the caller profile resolves");

    let layout = spec.layout.expect("the profile produces a layout");
    assert_eq!(layout.word_width, Some(16));
    assert_eq!(layout.field_map["kind"].width, 4);
}

#[test]
fn an_unknown_profile_names_the_known_ones() {
    let error = parse("target: t\nprofile: nope\nfields:\n  a:\n    range: [0, 1]\n")
        .expect_err("an unknown profile is rejected")
        .to_string();
    assert!(error.contains("unknown profile `nope`"), "{error}");
    #[cfg(feature = "riscv")]
    assert!(error.contains("riscv-r"), "{error}");
    #[cfg(not(feature = "riscv"))]
    assert!(error.contains("--features riscv"), "{error}");
}

#[test]
fn layout_and_profile_are_mutually_exclusive() {
    let error = parse(
        "target: t\nprofile: demo\nlayout:\n  word_width: 8\nfields:\n  a:\n    range: [0, 1]\n",
    )
    .expect_err("a spec cannot declare both")
    .to_string();
    assert!(error.contains("not both"), "{error}");
}
