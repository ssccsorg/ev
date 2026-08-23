//! Lib-level regression tests for the Tagma hardware fixture specs.
//!
//! The CLI tests in cli_test.rs exercise the same fixtures through the
//! spawned binary, which the coverage instrumentation does not see. These
//! tests evaluate the fixtures in-process through the public library API,
//! so the ge/le constraint path, the YAML parse of the hex boundary
//! constants, the tagma_decode projector, and the domain counts are covered
//! by the instrumented suite.

use ev::spec::VerificationSpec;
use ev::synth::GenerateRtl;
use ev::verify::compose::expand_all;
use ev::verify::evaluate::evaluate_all;
use ev::verify::registry::{ConstraintRegistry, ProjectorRegistry};

fn load_fixture(path: &str) -> VerificationSpec {
    VerificationSpec::from_yaml(std::path::Path::new(path))
        .unwrap_or_else(|e| panic!("failed to load fixture {path}: {e}"))
}

/// (code, projection) pairs for every passing combination, in domain order.
fn valid_code_projection_pairs(spec: &VerificationSpec) -> Vec<(i64, i64)> {
    let combos = expand_all(spec).expect("domain expansion must succeed");
    evaluate_all(
        spec,
        combos,
        &ConstraintRegistry::default(),
        &ProjectorRegistry::default(),
    )
    .into_iter()
    .filter(|r| r.passed)
    .map(|r| {
        (
            r.combination.values[0],
            r.projection.expect("valid code points must project"),
        )
    })
    .collect()
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

/// The tagma_decode projector packs the decomposition into the golden-anchor
/// layout offset[28:15] i[14:10] m[9:5] f[4:0]. Literal spot values pin the
/// packing without re-deriving it from the same formula.
#[test]
fn tagma_decode_projection_spot_values() {
    let spec = load_fixture("tests/fixtures/tagma/tagma_decoder.xif.yaml");
    let pairs = valid_code_projection_pairs(&spec);
    assert_eq!(pairs.len(), 11_172);

    // code 0xAC00: offset 0, i 0, m 0, f 0.
    assert_eq!(pairs[0], (0xAC00, 0));
    // code 0xAC00 + 587: offset 587, i 0, m 20, f 27 (last medial of the first initial).
    // i 0 contributes no bits to the pack (bit 10..14 are zero).
    assert_eq!(pairs[587], (0xAC00 + 587, (587 << 15) | (20 << 5) | 27));
    // code 0xD7A3: offset 11171, i 18, m 20, f 27 (last syllable).
    assert_eq!(
        pairs[11_171],
        (0xD7A3, (11_171 << 15) | (18 << 10) | (20 << 5) | 27)
    );
}

/// Over the whole valid domain, the projector equals the golden-anchor
/// contract: line k packs offset k with the i/m/f decomposition of k.
#[test]
fn tagma_decode_projection_full_domain() {
    let spec = load_fixture("tests/fixtures/tagma/tagma_decoder.xif.yaml");
    let pairs = valid_code_projection_pairs(&spec);
    assert_eq!(pairs.len(), 11_172);

    for (k, (code, proj)) in pairs.iter().enumerate() {
        let k = k as i64;
        assert_eq!(*code, 0xAC00 + k, "code at offset {k}");
        let expected = (k << 15) | ((k / 588) << 10) | (((k % 588) / 28) << 5) | (k % 28);
        assert_eq!(*proj, expected, "packed projection at offset {k}");
    }
}

/// The SV generator must emit the packed decode expression for the
/// tagma_decode projector, so the golden-anchor layout is preserved in the
/// generated RTL.
#[test]
fn tagma_decode_sv_generation() {
    let spec = load_fixture("tests/fixtures/tagma/tagma_decoder.xif.yaml");
    let sv_path = ev::synth::SvGenerator
        .generate(&spec)
        .expect("sv generation");
    let sv = std::fs::read_to_string(&sv_path).expect("read generated sv");

    assert!(
        sv.contains("((code - 44032) << 15)"),
        "generated SV must pack the offset into bits 28:15"
    );
    assert!(
        sv.contains("/ 588") && sv.contains("% 588") && sv.contains("% 28"),
        "generated SV must express the Tagma decomposition"
    );
}
