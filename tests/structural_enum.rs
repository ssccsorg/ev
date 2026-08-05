//! Regression tests for the structural enumeration pipeline.
//!
//! Guards the invariant that `StructuralEnum` emits exactly the combinations
//! that satisfy the full constraint set, matching `evaluate_all`. This covers
//! the cross-constraint wrap bug where a child field kept its stale value
//! after its parent advanced, and the initial-state bug where the raw
//! all-zero state was emitted without validation.

use std::collections::BTreeMap;

use ev::spec::{ConstraintSpec, FieldSpec, ProjectorSpec, VerificationSpec};
use ev::verify::compose::{expand_all, StructuralEnum};
use ev::verify::evaluate::{evaluate_all, validate_into_space};
use ev::verify::registry::{Check, ConstraintRegistry, ProjectorRegistry};

fn evaluate_valid_count(spec: &VerificationSpec) -> usize {
    let combos = expand_all(spec).unwrap();
    evaluate_all(
        spec,
        combos,
        &ConstraintRegistry::default(),
        &ProjectorRegistry::default(),
    )
    .into_iter()
    .filter(|r| r.passed)
    .count()
}

fn structural_valid_count(spec: &VerificationSpec) -> usize {
    let checks = ConstraintRegistry::default().build_all(&spec.constraints, &spec.fields);
    StructuralEnum::new(spec)
        .filter(|combo| checks.iter().all(|c| c.allows(combo.point.coordinates())))
        .count()
}

fn make_spec(
    fields: BTreeMap<String, FieldSpec>,
    constraints: Vec<ConstraintSpec>,
) -> VerificationSpec {
    VerificationSpec {
        target: "structural_enum_test".into(),
        fields,
        encoding: None,
        constraints,
        projector: ProjectorSpec::Sum,
    }
}

/// The exact wrap pattern that produced the bug: the first allowed child
/// value (0) is valid for parents 0 and 1 but invalid for parent 2.
#[test]
fn structural_enum_cross_wrap_regression() {
    let mut fields = BTreeMap::new();
    fields.insert(
        "a".into(),
        FieldSpec {
            range: Some((0, 2)),
            alignment: None,
            values: None,
        },
    );
    fields.insert(
        "b".into(),
        FieldSpec {
            range: Some((0, 7)),
            alignment: None,
            values: None,
        },
    );
    fields.insert(
        "c".into(),
        FieldSpec {
            range: Some((0, 1)),
            alignment: None,
            values: None,
        },
    );
    let mapping: std::collections::HashMap<i64, Vec<i64>> =
        [(0, vec![0, 1]), (1, vec![0, 1]), (2, vec![5])].into();
    let spec = make_spec(
        fields,
        vec![ConstraintSpec::Cross {
            field_a: "a".into(),
            field_b: "b".into(),
            mapping,
        }],
    );

    // a=0: b in {0,1} x c(2) = 4; a=1: 4; a=2: b in {5} x c(2) = 2. Total 10.
    let expected = 10usize;
    let emitted = StructuralEnum::new(&spec).count();
    assert_eq!(emitted, expected, "every emitted combination must be valid");
    assert_eq!(
        structural_valid_count(&spec),
        expected,
        "emitted set must equal the full-check valid set"
    );
    assert_eq!(evaluate_valid_count(&spec), expected);
}

/// The all-zero raw state is invalid; enumeration must start at the first
/// valid state rather than emitting the raw state.
#[test]
fn structural_enum_skips_invalid_initial_state() {
    let mut fields = BTreeMap::new();
    fields.insert(
        "a".into(),
        FieldSpec {
            range: Some((0, 1)),
            alignment: None,
            values: None,
        },
    );
    fields.insert(
        "b".into(),
        FieldSpec {
            range: Some((0, 7)),
            alignment: None,
            values: None,
        },
    );
    let mapping: std::collections::HashMap<i64, Vec<i64>> = [(0, vec![5]), (1, vec![0, 1])].into();
    let spec = make_spec(
        fields,
        vec![ConstraintSpec::Cross {
            field_a: "a".into(),
            field_b: "b".into(),
            mapping,
        }],
    );

    // a=0: b=5 (1); a=1: b in {0,1} (2). Total 3. The all-zero state (a=0,b=0)
    // is invalid and must never be emitted.
    let expected = 3usize;
    let emitted = StructuralEnum::new(&spec).count();
    assert_eq!(emitted, expected);
    assert_eq!(structural_valid_count(&spec), expected);
    assert_eq!(evaluate_valid_count(&spec), expected);
}

fn load_fixture(path: &str) -> VerificationSpec {
    VerificationSpec::from_yaml(std::path::Path::new(path))
        .unwrap_or_else(|e| panic!("failed to load fixture {path}: {e}"))
}

/// CVA6 full fixture: struct_enum must emit exactly the 229,376 valid
/// combinations, all satisfying the full constraint set. The expected count
/// is the `evaluate_all` result measured on the committed fixture; running
/// the full 33.5M evaluation here would add ~20 s to the suite.
#[test]
fn structural_enum_matches_evaluate_cva6_full() {
    let spec = load_fixture("tests/fixtures/cva6/xif_ref.xif.yaml");
    let expected = 229_376usize;

    let emitted = StructuralEnum::new(&spec).count();
    assert_eq!(
        emitted, expected,
        "struct_enum emitted {emitted}, expected {expected}"
    );
    assert_eq!(
        structural_valid_count(&spec),
        expected,
        "every emitted combination must satisfy the full constraint set"
    );
}

/// CVA6 R4 fixture: struct_enum must emit exactly the 12,288 valid
/// combinations.
#[test]
fn structural_enum_matches_evaluate_cva6_r4() {
    let spec = load_fixture("tests/fixtures/cva6/xif_ref_r4.xif.yaml");
    let expected = 12_288usize;

    let emitted = StructuralEnum::new(&spec).count();
    assert_eq!(
        emitted, expected,
        "struct_enum emitted {emitted}, expected {expected}"
    );
    assert_eq!(structural_valid_count(&spec), expected);
}

/// validate_into_space must place exactly the distinct valid coordinate paths
/// into the CoordSpace, with no invalid combinations inserted. The CoordSpace
/// is keyed by coordinates, so combinations that `enable_mask` collapses to
/// the same path (e.g. the 32,768 CUS_ADD combinations that all become
/// funct3=1, funct7=0, rs1=rs2=rd=0) are stored once. The expected counts are
/// the distinct-path counts measured on the committed fixtures.
#[test]
fn validate_into_space_matches_evaluate_cva6_fixtures() {
    for (path, expected_distinct) in [
        ("tests/fixtures/cva6/xif_ref.xif.yaml", 164_865usize),
        ("tests/fixtures/cva6/xif_ref_r4.xif.yaml", 8_708usize),
    ] {
        let spec = load_fixture(path);
        let space = validate_into_space(&spec, &ConstraintRegistry::default());
        assert_eq!(
            space.iter().count(),
            expected_distinct,
            "validate_into_space must store each distinct valid path once for {path}"
        );

        // The stored set must equal the distinct paths of the structurally
        // valid combinations.
        let distinct_valid: std::collections::HashSet<Vec<i64>> = StructuralEnum::new(&spec)
            .map(|c| c.values.clone())
            .collect();
        assert_eq!(
            space.iter().count(),
            distinct_valid.len(),
            "validate_into_space path count must match the distinct valid paths for {path}"
        );
    }
}
