//! Cross-channel check: the `decompose` projector against the Tagma
//! reference engine.
//!
//! `tests/tagma_fixture.rs` pins the projector two ways: three literal anchor
//! values, written without the projector's constants, and a full-domain
//! comparison against the decomposition formula restated in Rust. The
//! restated formula is self-consistent, so a stride or packing error shared
//! by the formula and the projector survives it. This file adds the
//! independent channel: the same projections computed by
//! `tagma_core::Coord::to_axes`, the reference engine syntagma uses to
//! export `hw/rtl/golden_anchors.hex`. A regression on either side becomes a
//! disagreement instead of a green run.
//!
//! Layout under test, shared with the syntagma anchor file:
//! `offset[28:15] i[14:10] m[9:5] f[4:0]`.
//!
//! The anchor file itself is a generated artifact in syntagma
//! (`hw/.gitignore`, `make golden-export`), so the reference engine is the
//! channel that is always available. When a generated file is at hand, point
//! `EV_TAGMA_ANCHORS` at it and it is checked line by line as well.

use ev::spec::VerificationSpec;
use ev::verify::compose::expand_all;
use ev::verify::evaluate::evaluate_all;
use ev::verify::registry::{ConstraintRegistry, ProjectorRegistry};
use tagma_core::Coord;

/// Base code point of the Hangul syllable block (U+AC00).
const CODE_BASE: i64 = 0xAC00;
/// Valid syllables: 19 x 21 x 28.
const N_VALID: usize = 11_172;
/// Axis bounds of the reference engine (`tagma_core::Coord`).
const N_INIT: u8 = 19;
const N_MED: u8 = 21;
const N_FIN: u8 = 28;
/// Bit positions of the anchor layout.
const SHIFT_OFFSET: i64 = 15;
const SHIFT_INIT: i64 = 10;
const SHIFT_MED: i64 = 5;
/// Environment variable pointing at a generated `golden_anchors.hex`.
const ANCHOR_ENV: &str = "EV_TAGMA_ANCHORS";

fn load_fixture(path: &str) -> VerificationSpec {
    VerificationSpec::from_yaml(std::path::Path::new(path))
        .unwrap_or_else(|e| panic!("failed to load fixture {path}: {e}"))
}

/// (code, projection) for every passing combination, in domain order, so the
/// index of an entry is its offset from `CODE_BASE`. The order is what lets
/// the anchor file be checked line by line, so it is asserted here rather
/// than in one of the two channels.
fn valid_code_projection_pairs() -> Vec<(i64, i64)> {
    let spec = load_fixture("tests/fixtures/tagma/tagma_decoder.xif.yaml");
    let combos = expand_all(&spec).expect("domain expansion must succeed");
    let pairs: Vec<(i64, i64)> = evaluate_all(
        &spec,
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
    .collect();

    assert_eq!(pairs.len(), N_VALID, "valid code point count");
    for (offset, (code, _)) in pairs.iter().enumerate() {
        assert_eq!(*code, CODE_BASE + offset as i64, "code at offset {offset}");
    }
    pairs
}

/// Pack the reference axes into the syntagma anchor layout.
fn pack(offset: i64, initial: u8, medial: u8, final_: u8) -> i64 {
    (offset << SHIFT_OFFSET)
        | ((initial as i64) << SHIFT_INIT)
        | ((medial as i64) << SHIFT_MED)
        | final_ as i64
}

/// Anchor-layout word for one offset, computed by the reference engine.
fn reference_pack(offset: i64) -> i64 {
    let coord = Coord::new(offset as u16)
        .unwrap_or_else(|| panic!("offset {offset} must be a valid coordinate"));
    let (initial, medial, final_) = coord.to_axes();
    assert!(
        initial < N_INIT && medial < N_MED && final_ < N_FIN,
        "offset {offset}: axes ({initial}, {medial}, {final_}) exceed the \
         reference bounds ({N_INIT}, {N_MED}, {N_FIN})"
    );
    pack(offset, initial, medial, final_)
}

/// Path of the generated anchor file to check, if one was supplied.
fn anchor_file() -> Option<String> {
    match std::env::var(ANCHOR_ENV) {
        Ok(path) if !path.trim().is_empty() => Some(path),
        _ => None,
    }
}

/// Channel 1: the projector must equal the reference engine on every valid
/// code point.
#[test]
fn decompose_matches_reference_engine_full_domain() {
    let pairs = valid_code_projection_pairs();

    for (offset, (_code, projection)) in pairs.iter().enumerate() {
        assert_eq!(
            *projection,
            reference_pack(offset as i64),
            "projection disagrees with tagma_core at offset {offset}"
        );
    }
}

/// Channel 2: when a generated `golden_anchors.hex` is supplied, file line
/// k + 1 (the file has no header) must equal the projection of code
/// `0xAC00 + k`.
#[test]
fn decompose_matches_golden_anchor_file() {
    let Some(path) = anchor_file() else {
        eprintln!("{ANCHOR_ENV} is unset or empty; the anchor-file channel did not run");
        return;
    };

    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read anchor file {path}: {e}"));
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(
        lines.len(),
        N_VALID,
        "anchor file {path} must hold one entry per valid code point"
    );

    let pairs = valid_code_projection_pairs();
    for (k, line) in lines.iter().enumerate() {
        let field = line.trim();
        let value = i64::from_str_radix(field, 16).unwrap_or_else(|e| {
            panic!("anchor file {path} line {}: not hex ({field}): {e}", k + 1)
        });
        assert_eq!(
            value,
            pairs[k].1,
            "anchor file {path} line {} disagrees with the projection of code 0x{:X}",
            k + 1,
            CODE_BASE + k as i64
        );
    }
}
