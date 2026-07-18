//! Evaluation — runs observation on each constraint combination.
//!
//! Uses pluggable checks resolved from registries.

use crate::spec::{ConstraintSpec, VerificationSpec};
use crate::verify::compose::{coords_to_coord_vec, Combination, EnumerateIter};
use crate::verify::registry::{Check, ConstraintRegistry, ProjectorRegistry};

/// Result of evaluating a single constraint combination.
#[derive(Debug, Clone)]
pub struct Evaluation {
    pub combination: Combination,
    pub passed: bool,
    pub projection: Option<i64>,
    pub reason: String,
}

/// Build a list of checks from the spec, excluding enable_mask constraints.
fn build_checks(spec: &VerificationSpec, registry: &ConstraintRegistry) -> Vec<Box<dyn Check>> {
    let mut checks: Vec<Box<dyn Check>> = Vec::new();

    let regular_constraints: Vec<ConstraintSpec> = spec
        .constraints
        .iter()
        .filter(|c| !matches!(c, ConstraintSpec::EnableMask { .. }))
        .cloned()
        .collect();

    for c in registry.build_all(&regular_constraints, &spec.fields) {
        checks.push(c.into_check());
    }

    checks
}

/// Evaluate all combinations using the given registries.
pub fn evaluate_all(
    spec: &VerificationSpec,
    combinations: Vec<Combination>,
    constraint_registry: &ConstraintRegistry,
    projector_registry: &ProjectorRegistry,
) -> Vec<Evaluation> {
    let checks = build_checks(spec, constraint_registry);
    let evaluator = projector_registry
        .resolve(&spec.projector, &spec.fields)
        .expect("projector type must be registered");

    combinations
        .into_iter()
        .map(|combination| {
            // enable_mask has already been applied by expand_all() in compose.rs.
            // The combination's values, coordinates, and point reflect masked fields.
            // Check field domain validity
            for (axis, (name, field_spec)) in spec.fields.iter().enumerate() {
                if let Some(value) = combination.coordinates.get_axis(axis) {
                    if !field_spec.allows(value) {
                        return Evaluation {
                            combination,
                            passed: false,
                            projection: None,
                            reason: format!(
                                "{}={} (expected {})",
                                name,
                                value,
                                describe_field(field_spec)
                            ),
                        };
                    }
                }
            }

            // Check all constraints (field-agnostic)
            for check in &checks {
                if !check.allows(combination.point.coordinates()) {
                    return Evaluation {
                        combination,
                        passed: false,
                        projection: None,
                        reason: check.describe(),
                    };
                }
            }

            let projection = evaluator.evaluate(&combination.point);

            Evaluation {
                combination,
                passed: true,
                projection,
                reason: String::new(),
            }
        })
        .collect()
}

/// Validate all combinations into a DynCoordSpace.
///
/// Only valid combinations are stored in the CoordSpace.
/// Invalid combinations are structurally absent (vacant slots),
/// detectable in 1.65 ns via DynCoordSpace lookup.
///
/// Uses EnumerateIter for O(1) memory iteration — no Vec allocation.
pub fn validate_into_space(
    spec: &VerificationSpec,
    constraint_registry: &ConstraintRegistry,
) -> tagma_core::DynCoordSpace<()> {
    use tagma_core::DynCoordSpace;

    let checks = constraint_registry.build_all(&spec.constraints, &spec.fields);
    let field_list: Vec<&String> = spec.fields.keys().collect();
    let mut space: DynCoordSpace<()> = DynCoordSpace::new();

    for combo in EnumerateIter::new(spec) {
        let coords = &combo.coordinates;

        // Check field domain validity
        let mut valid = true;
        for (axis, name) in field_list.iter().enumerate() {
            if let Some(value) = coords.get_axis(axis) {
                if !spec.fields.get(*name).unwrap().allows(value) {
                    valid = false;
                    break;
                }
            }
        }
        if !valid {
            continue;
        }

        // Check all constraints
        let mut passes = true;
        for check in &checks {
            if !check.allows(combo.point.coordinates()) {
                passes = false;
                break;
            }
        }
        if !passes {
            continue;
        }

        // Valid: store in CoordSpace
        if let Some(path) = coords_to_coord_vec(&coords.raw) {
            let _ = space.place(&path, ());
        }
    }

    space
}

fn describe_field(field_spec: &crate::spec::FieldSpec) -> String {
    if let Some(ref vals) = field_spec.values {
        format!(
            "{{{}}}",
            vals.iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )
    } else if let Some((min, max)) = field_spec.range {
        let step = field_spec.alignment.unwrap_or(1);
        if step == 1 {
            format!("{}..={}", min, max)
        } else {
            format!("{}..={} step {}", min, max, step)
        }
    } else {
        "any".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::{ConstraintSpec, FieldSpec, ProjectorSpec};
    use std::collections::BTreeMap;

    fn make_spec(
        fields: BTreeMap<String, FieldSpec>,
        constraints: Vec<ConstraintSpec>,
        projector: ProjectorSpec,
    ) -> VerificationSpec {
        VerificationSpec {
            target: "test".into(),
            fields,
            constraints,
            projector,
            encoding: None,
        }
    }

    fn make_single_field_spec(value: i64) -> (VerificationSpec, Vec<Combination>) {
        let mut fields = BTreeMap::new();
        fields.insert(
            "x".into(),
            FieldSpec {
                range: None,
                alignment: None,
                values: Some(vec![value]),
            },
        );
        let spec = make_spec(
            fields,
            vec![],
            ProjectorSpec::Identity { field: "x".into() },
        );
        let combos = crate::verify::compose::expand_all(&spec).expect("expand should succeed");
        (spec, combos)
    }

    // ── Basic pass/fail ───────────────────────────────────────────

    #[test]
    fn all_pass() {
        let (spec, combos) = make_single_field_spec(42);
        let results = evaluate_all(
            &spec,
            combos,
            &ConstraintRegistry::default(),
            &ProjectorRegistry::default(),
        );
        assert_eq!(results.len(), 1);
        assert!(results[0].passed);
        assert_eq!(results[0].projection, Some(42));
        assert!(results[0].reason.is_empty());
    }

    #[test]
    fn out_of_range_value_fails() {
        let mut fields = BTreeMap::new();
        fields.insert(
            "x".into(),
            FieldSpec {
                range: Some((0, 10)),
                alignment: None,
                values: None,
            },
        );
        // expand_all will only produce values 0..=10, so we manually
        // construct a combination with an out-of-range value.
        let spec = make_spec(
            fields,
            vec![],
            ProjectorSpec::Identity { field: "x".into() },
        );
        let coord = crate::verify::compose::Coordinates::new(vec![20]);
        let point = crate::verify::compose::Point::new(coord.clone());
        let combo = crate::verify::compose::Combination {
            values: vec![20],
            coordinates: coord,
            point,
        };
        let results = evaluate_all(
            &spec,
            vec![combo],
            &ConstraintRegistry::default(),
            &ProjectorRegistry::default(),
        );
        assert_eq!(results.len(), 1);
        assert!(!results[0].passed);
        assert!(
            results[0].reason.contains("20"),
            "reason should mention the bad value: {}",
            results[0].reason
        );
    }

    // ── Eq constraint ─────────────────────────────────────────────

    #[test]
    fn eq_constraint_allows_equal() {
        let mut fields = BTreeMap::new();
        fields.insert(
            "a".into(),
            FieldSpec {
                range: None,
                alignment: None,
                values: Some(vec![5]),
            },
        );
        fields.insert(
            "b".into(),
            FieldSpec {
                range: None,
                alignment: None,
                values: Some(vec![5]),
            },
        );
        let spec = make_spec(
            fields,
            vec![ConstraintSpec::Eq {
                field_a: "a".into(),
                field_b: "b".into(),
            }],
            ProjectorSpec::Sum,
        );
        let combos = crate::verify::compose::expand_all(&spec).unwrap();
        assert_eq!(combos.len(), 1, "only one combo: a=b=5");
        let results = evaluate_all(
            &spec,
            combos,
            &ConstraintRegistry::default(),
            &ProjectorRegistry::default(),
        );
        assert!(results[0].passed, "a=b should pass");
        assert_eq!(results[0].projection, Some(10), "sum 5+5 = 10");
    }

    #[test]
    fn eq_constraint_rejects_unequal() {
        let mut fields = BTreeMap::new();
        fields.insert(
            "a".into(),
            FieldSpec {
                range: None,
                alignment: None,
                values: Some(vec![3]),
            },
        );
        fields.insert(
            "b".into(),
            FieldSpec {
                range: None,
                alignment: None,
                values: Some(vec![7]),
            },
        );
        let spec = make_spec(
            fields,
            vec![ConstraintSpec::Eq {
                field_a: "a".into(),
                field_b: "b".into(),
            }],
            ProjectorSpec::Sum,
        );
        let combos = crate::verify::compose::expand_all(&spec).unwrap();
        assert_eq!(combos.len(), 1, "one combo but values differ");
        let results = evaluate_all(
            &spec,
            combos,
            &ConstraintRegistry::default(),
            &ProjectorRegistry::default(),
        );
        assert!(!results[0].passed, "a=3, b=7 should fail eq");
        assert!(results[0].reason.contains("=="), "reason should mention eq");
    }

    // ── Even + Range constraint ───────────────────────────────────

    #[test]
    fn even_and_range_constraint() {
        let mut fields = BTreeMap::new();
        fields.insert(
            "coord".into(),
            FieldSpec {
                range: None,
                alignment: None,
                values: Some(vec![2, 3, 10]),
            },
        );
        let spec = make_spec(
            fields,
            vec![
                ConstraintSpec::Even {
                    field: "coord".into(),
                },
                ConstraintSpec::Range {
                    field: "coord".into(),
                    min: 0,
                    max: 10,
                },
            ],
            ProjectorSpec::Identity {
                field: "coord".into(),
            },
        );
        let combos = crate::verify::compose::expand_all(&spec).unwrap();
        assert_eq!(combos.len(), 3, "3 field values");
        let results = evaluate_all(
            &spec,
            combos,
            &ConstraintRegistry::default(),
            &ProjectorRegistry::default(),
        );
        // 2: even(ok) + range(ok) → pass, projection=2
        // 3: even(fail) → reject
        // 10: even(ok) + range(ok) → pass, projection=10
        for r in &results {
            match r.combination.values[0] {
                2 => {
                    assert!(r.passed, "2 should pass");
                    assert_eq!(r.projection, Some(2));
                }
                3 => {
                    assert!(!r.passed, "3 should fail (odd)");
                }
                10 => {
                    assert!(r.passed, "10 should pass");
                    assert_eq!(r.projection, Some(10));
                }
                v => panic!("unexpected value: {}", v),
            }
        }
    }

    // ── New constraint types ──────────────────────────────────────

    #[test]
    fn neq_constraint_allows_unequal() {
        let mut fields = BTreeMap::new();
        fields.insert(
            "a".into(),
            FieldSpec {
                range: None,
                alignment: None,
                values: Some(vec![3]),
            },
        );
        fields.insert(
            "b".into(),
            FieldSpec {
                range: None,
                alignment: None,
                values: Some(vec![7]),
            },
        );
        let spec = make_spec(
            fields,
            vec![ConstraintSpec::Neq {
                field_a: "a".into(),
                field_b: "b".into(),
            }],
            ProjectorSpec::Sum,
        );
        let combos = crate::verify::compose::expand_all(&spec).unwrap();
        assert_eq!(combos.len(), 1);
        let results = evaluate_all(
            &spec,
            combos,
            &ConstraintRegistry::default(),
            &ProjectorRegistry::default(),
        );
        assert!(results[0].passed, "a=3, b=7 should pass neq");
    }

    #[test]
    fn neq_constraint_rejects_equal() {
        let mut fields = BTreeMap::new();
        fields.insert(
            "a".into(),
            FieldSpec {
                range: None,
                alignment: None,
                values: Some(vec![5]),
            },
        );
        fields.insert(
            "b".into(),
            FieldSpec {
                range: None,
                alignment: None,
                values: Some(vec![5]),
            },
        );
        let spec = make_spec(
            fields,
            vec![ConstraintSpec::Neq {
                field_a: "a".into(),
                field_b: "b".into(),
            }],
            ProjectorSpec::Sum,
        );
        let combos = crate::verify::compose::expand_all(&spec).unwrap();
        let results = evaluate_all(
            &spec,
            combos,
            &ConstraintRegistry::default(),
            &ProjectorRegistry::default(),
        );
        assert!(!results[0].passed, "a=5, b=5 should fail neq");
        assert!(results[0].reason.contains("!="), "reason should mention !=");
    }

    #[test]
    fn lt_constraint() {
        let mut fields = BTreeMap::new();
        fields.insert(
            "x".into(),
            FieldSpec {
                range: None,
                alignment: None,
                values: Some(vec![1, 5, 10]),
            },
        );
        let spec = make_spec(
            fields,
            vec![ConstraintSpec::Lt {
                field: "x".into(),
                value: 5,
            }],
            ProjectorSpec::Identity { field: "x".into() },
        );
        let combos = crate::verify::compose::expand_all(&spec).unwrap();
        assert_eq!(combos.len(), 3);
        let results = evaluate_all(
            &spec,
            combos,
            &ConstraintRegistry::default(),
            &ProjectorRegistry::default(),
        );
        assert!(results[0].passed, "1 < 5 should pass");
        assert!(!results[1].passed, "5 < 5 should fail");
        assert!(!results[2].passed, "10 < 5 should fail");
    }

    #[test]
    fn gt_constraint() {
        let mut fields = BTreeMap::new();
        fields.insert(
            "x".into(),
            FieldSpec {
                range: None,
                alignment: None,
                values: Some(vec![1, 5, 10]),
            },
        );
        let spec = make_spec(
            fields,
            vec![ConstraintSpec::Gt {
                field: "x".into(),
                value: 5,
            }],
            ProjectorSpec::Identity { field: "x".into() },
        );
        let combos = crate::verify::compose::expand_all(&spec).unwrap();
        let results = evaluate_all(
            &spec,
            combos,
            &ConstraintRegistry::default(),
            &ProjectorRegistry::default(),
        );
        assert!(!results[0].passed, "1 > 5 should fail");
        assert!(!results[1].passed, "5 > 5 should fail");
        assert!(results[2].passed, "10 > 5 should pass");
    }

    #[test]
    fn le_constraint() {
        let mut fields = BTreeMap::new();
        fields.insert(
            "x".into(),
            FieldSpec {
                range: None,
                alignment: None,
                values: Some(vec![1, 5, 10]),
            },
        );
        let spec = make_spec(
            fields,
            vec![ConstraintSpec::Le {
                field: "x".into(),
                value: 5,
            }],
            ProjectorSpec::Identity { field: "x".into() },
        );
        let combos = crate::verify::compose::expand_all(&spec).unwrap();
        let results = evaluate_all(
            &spec,
            combos,
            &ConstraintRegistry::default(),
            &ProjectorRegistry::default(),
        );
        assert!(results[0].passed, "1 <= 5 should pass");
        assert!(results[1].passed, "5 <= 5 should pass");
        assert!(!results[2].passed, "10 <= 5 should fail");
    }

    #[test]
    fn ge_constraint() {
        let mut fields = BTreeMap::new();
        fields.insert(
            "x".into(),
            FieldSpec {
                range: None,
                alignment: None,
                values: Some(vec![1, 5, 10]),
            },
        );
        let spec = make_spec(
            fields,
            vec![ConstraintSpec::Ge {
                field: "x".into(),
                value: 5,
            }],
            ProjectorSpec::Identity { field: "x".into() },
        );
        let combos = crate::verify::compose::expand_all(&spec).unwrap();
        let results = evaluate_all(
            &spec,
            combos,
            &ConstraintRegistry::default(),
            &ProjectorRegistry::default(),
        );
        assert!(!results[0].passed, "1 >= 5 should fail");
        assert!(results[1].passed, "5 >= 5 should pass");
        assert!(results[2].passed, "10 >= 5 should pass");
    }

    #[test]
    fn oneof_constraint() {
        let mut fields = BTreeMap::new();
        fields.insert(
            "x".into(),
            FieldSpec {
                range: None,
                alignment: None,
                values: Some(vec![2, 4, 7]),
            },
        );
        let spec = make_spec(
            fields,
            vec![ConstraintSpec::Oneof {
                field: "x".into(),
                values: vec![0, 2, 4],
            }],
            ProjectorSpec::Identity { field: "x".into() },
        );
        let combos = crate::verify::compose::expand_all(&spec).unwrap();
        assert_eq!(combos.len(), 3);
        let results = evaluate_all(
            &spec,
            combos,
            &ConstraintRegistry::default(),
            &ProjectorRegistry::default(),
        );
        assert!(results[0].passed, "2 in set should pass");
        assert!(results[1].passed, "4 in set should pass");
        assert!(!results[2].passed, "7 not in set should fail");
    }

    #[test]
    fn bitmask_constraint() {
        let mut fields = BTreeMap::new();
        fields.insert(
            "a".into(),
            FieldSpec {
                range: None,
                alignment: None,
                values: Some(vec![0, 1, 2, 3]),
            },
        );
        fields.insert(
            "b".into(),
            FieldSpec {
                range: None,
                alignment: None,
                values: Some(vec![0, 1, 2, 3]),
            },
        );
        let spec = make_spec(
            fields,
            vec![ConstraintSpec::Bitmask {
                field: "a".into(),
                mask: 2,
                value: 2,
            }],
            ProjectorSpec::Sum,
        );
        let combos = crate::verify::compose::expand_all(&spec).unwrap();
        assert_eq!(combos.len(), 16); // 4 × 4
        let results = evaluate_all(
            &spec,
            combos,
            &ConstraintRegistry::default(),
            &ProjectorRegistry::default(),
        );
        for r in &results {
            let a = r.combination.values[0];
            let b = r.combination.values[1];
            match a {
                0 | 1 => {
                    // 0 & 2 = 0 ≠ 2, 1 & 2 = 0 ≠ 2
                    assert!(!r.passed, "a={}, b={} should fail (bit 1 not set)", a, b);
                }
                2 | 3 => {
                    // 2 & 2 = 2, 3 & 2 = 2
                    assert!(r.passed, "a={}, b={} should pass (bit 1 set)", a, b);
                    assert_eq!(r.projection, Some(a + b));
                }
                _ => unreachable!(),
            }
        }
    }

    #[test]
    fn cross_constraint() {
        let mut fields = BTreeMap::new();
        fields.insert(
            "op".into(),
            FieldSpec {
                range: None,
                alignment: None,
                values: Some(vec![0, 1, 2]),
            },
        );
        fields.insert(
            "sub".into(),
            FieldSpec {
                range: None,
                alignment: None,
                values: Some(vec![0, 1, 2, 3]),
            },
        );
        let mapping: std::collections::HashMap<i64, Vec<i64>> =
            [(0, vec![0]), (1, vec![0, 1, 2])].into();
        let spec = make_spec(
            fields,
            vec![ConstraintSpec::Cross {
                field_a: "op".into(),
                field_b: "sub".into(),
                mapping,
            }],
            ProjectorSpec::Identity { field: "op".into() },
        );
        let combos = crate::verify::compose::expand_all(&spec).unwrap();
        // 3 × 4 = 12 raw combinations
        assert_eq!(combos.len(), 12);
        let results = evaluate_all(
            &spec,
            combos,
            &ConstraintRegistry::default(),
            &ProjectorRegistry::default(),
        );
        // op=0, sub=0: passes (mapped, sub in allowed)
        // op=0, sub=1,2,3: fails (sub not in [0])
        // op=1, sub=0,1,2: passes (mapped, sub in [0,1,2])
        // op=1, sub=3: fails (3 not in [0,1,2])
        // op=2: passes trivially (not in mapping, unrestrict)
        for r in &results {
            let op = r.combination.values[0];
            let sub = r.combination.values[1];
            match (op, sub) {
                (0, 0) => assert!(r.passed, "op=0, sub=0 should pass"),
                (0, _) => assert!(!r.passed, "op=0, sub={} should fail", sub),
                (1, 0..=2) => assert!(r.passed, "op=1, sub={} should pass", sub),
                (1, 3) => assert!(!r.passed, "op=1, sub=3 should fail"),
                (2, _) => assert!(r.passed, "op=2 (unmapped) should pass"),
                _ => {}
            }
        }
    }

    #[test]
    fn validate_into_space_matches_evaluate_all() {
        // validate_into_space should produce the same valid set as evaluate_all
        let mut fields = BTreeMap::new();
        fields.insert(
            "op".into(),
            FieldSpec {
                range: None,
                alignment: None,
                values: Some(vec![0, 1, 2]),
            },
        );
        fields.insert(
            "sub".into(),
            FieldSpec {
                range: None,
                alignment: None,
                values: Some(vec![0, 1, 2, 3]),
            },
        );
        let mapping: std::collections::HashMap<i64, Vec<i64>> =
            [(0, vec![0]), (1, vec![0, 1, 2])].into();
        let spec = make_spec(
            fields,
            vec![ConstraintSpec::Cross {
                field_a: "op".into(),
                field_b: "sub".into(),
                mapping,
            }],
            ProjectorSpec::Sum,
        );

        // Reference: evaluate_all with Vec
        let combos = crate::verify::compose::expand_all(&spec).unwrap();
        let results = evaluate_all(
            &spec,
            combos,
            &ConstraintRegistry::default(),
            &ProjectorRegistry::default(),
        );
        let valid_count_ref = results.iter().filter(|r| r.passed).count();

        // CoordSpace: validate_into_space
        let space = validate_into_space(&spec, &ConstraintRegistry::default());
        let valid_count_cs = space.iter().count();

        assert_eq!(
            valid_count_cs, valid_count_ref,
            "validate_into_space should match evaluate_all valid count"
        );
    }

    // ── Edge cases ────────────────────────────────────────────────

    // ── EnableMask ────────────────────────────────────────────────

    #[test]
    fn enable_mask_forces_zero_when_trigger_matches() {
        let mut fields = BTreeMap::new();
        fields.insert(
            "op".into(),
            FieldSpec {
                range: None,
                alignment: None,
                values: Some(vec![0, 1]),
            },
        );
        fields.insert(
            "rs1".into(),
            FieldSpec {
                range: None,
                alignment: None,
                values: Some(vec![0, 1, 2, 3]),
            },
        );
        fields.insert(
            "rd".into(),
            FieldSpec {
                range: None,
                alignment: None,
                values: Some(vec![0, 1, 2, 3]),
            },
        );
        // When op=1 (NOP), force rs1=0 and rd=0.
        let spec = make_spec(
            fields,
            vec![ConstraintSpec::EnableMask {
                field: "op".into(),
                value: 1,
                disable: vec!["rs1".into(), "rd".into()],
            }],
            ProjectorSpec::Sum,
        );
        let combos = crate::verify::compose::expand_all(&spec).unwrap();
        // 2 × 4 × 4 = 32 raw combinations
        assert_eq!(combos.len(), 32);
        let results = evaluate_all(
            &spec,
            combos,
            &ConstraintRegistry::default(),
            &ProjectorRegistry::default(),
        );
        assert_eq!(results.len(), 32);
        for r in &results {
            let op = r.combination.values[0];
            let rs1 = r.combination.values[1];
            let rd = r.combination.values[2];
            match op {
                0 => {
                    // op=0: no mask applied, any value allowed
                    assert!(r.passed, "op=0, rs1={}, rd={} should pass", rs1, rd);
                }
                1 => {
                    // op=1: enable_mask forces rs1=0, rd=0
                    assert!(
                        r.passed,
                        "op=1, rs1={}, rd={} should pass (all zero after mask)",
                        rs1, rd
                    );
                    assert_eq!(rs1, 0, "rs1 should be forced to 0 when op=1, got {}", rs1);
                    assert_eq!(rd, 0, "rd should be forced to 0 when op=1, got {}", rd);
                }
                _ => unreachable!(),
            }
        }
    }

    #[test]
    fn empty_combinations_returns_empty() {
        let (spec, _) = make_single_field_spec(1);
        let results = evaluate_all(
            &spec,
            vec![],
            &ConstraintRegistry::default(),
            &ProjectorRegistry::default(),
        );
        assert!(results.is_empty());
    }
}
