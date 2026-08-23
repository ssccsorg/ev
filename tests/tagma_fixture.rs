//! Lib-level regression tests for the Tagma hardware fixture specs.
//!
//! The CLI tests in cli_test.rs exercise the same fixtures through the
//! spawned binary, which the coverage instrumentation does not see. These
//! tests evaluate the fixtures in-process through the public library API,
//! so the ge/le constraint path, the YAML parse of the hex boundary
//! constants, and the domain counts are covered by the instrumented suite.

use ev::spec::VerificationSpec;
use ev::verify::compose::expand_all;
use ev::verify::evaluate::evaluate_all;
use ev::verify::registry::{ConstraintRegistry, ProjectorRegistry};

fn load_fixture(path: &str) -> VerificationSpec {
    VerificationSpec::from_yaml(std::path::Path::new(path))
        .unwrap_or_else(|e| panic!("failed to load fixture {path}: {e}"))
}

/// The decoder accepts exactly the 11,172 Hangul syllables in
/// [0xAC00, 0xD7A3]. The boundary assertion pins the correction recorded in
/// the syntagma verification devlog: the last valid syllable is U+D7A3, not
/// U+D7AF (0xAC00 + 11171 = 0xD7A3).
#[test]
fn tagma_decoder_domain_boundary() {
    let spec = load_fixture("tests/fixtures/tagma/tagma_decoder.xif.yaml");
    let combos = expand_all(&spec).expect("domain expansion must succeed");
    assert_eq!(
        combos.len(),
        65_536,
        "all 16-bit code points are raw candidates"
    );

    let results = evaluate_all(
        &spec,
        combos,
        &ConstraintRegistry::default(),
        &ProjectorRegistry::default(),
    );
    let valid: Vec<i64> = results
        .into_iter()
        .filter(|r| r.passed)
        .map(|r| r.combination.values[0])
        .collect();

    assert_eq!(
        valid.len(),
        11_172,
        "exactly the Hangul syllable block is valid"
    );
    assert_eq!(
        *valid.first().expect("non-empty"),
        0xAC00,
        "first valid code point"
    );
    assert_eq!(
        *valid.last().expect("non-empty"),
        0xD7A3,
        "last valid code point is U+D7A3"
    );
}

/// The demo top output space is the product of the axis ranges: every axis
/// triple is reachable, so the 11,172 combinations all pass.
#[test]
fn tagma_demo_top_output_space() {
    let spec = load_fixture("tests/fixtures/tagma/tagma_demo_top.xif.yaml");
    let combos = expand_all(&spec).expect("domain expansion must succeed");
    assert_eq!(combos.len(), 11_172, "19 x 21 x 28 axis triples");

    let results = evaluate_all(
        &spec,
        combos,
        &ConstraintRegistry::default(),
        &ProjectorRegistry::default(),
    );
    assert_eq!(results.len(), 11_172);
    assert!(
        results.iter().all(|r| r.passed),
        "every axis triple is reachable"
    );
}
