//! Evaluation — runs observation on each constraint combination.
//!
//! Uses pluggable checks resolved from registries.

use crate::compose::Combination;
use crate::registry::{Check, ConstraintRegistry, ProjectorRegistry};
use crate::spec::VerificationSpec;

/// Result of evaluating a single constraint combination.
#[derive(Debug, Clone)]
pub struct Evaluation {
    pub combination: Combination,
    pub passed: bool,
    pub projection: Option<i64>,
    pub reason: String,
}

/// Build a list of checks from the spec.
fn build_checks(spec: &VerificationSpec, registry: &ConstraintRegistry) -> Vec<Box<dyn Check>> {
    let mut checks: Vec<Box<dyn Check>> = Vec::new();

    for c in registry.build_all(&spec.constraints) {
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
        .build(&spec.projector)
        .expect("projector type must be registered");

    combinations
        .into_iter()
        .map(|combination| {
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

fn describe_field(fs: &crate::spec::FieldSpec) -> String {
    if let Some(ref vals) = fs.values {
        format!(
            "{{{}}}",
            vals.iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )
    } else if let Some((min, max)) = fs.range {
        let step = fs.alignment.unwrap_or(1);
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
        let spec = make_spec(fields, vec![], ProjectorSpec::Identity { axis: 0 });
        let combos = crate::compose::expand_all(&spec).expect("expand should succeed");
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
        let spec = make_spec(fields, vec![], ProjectorSpec::Identity { axis: 0 });
        let coord = crate::compose::Coordinates::new(vec![20]);
        let point = crate::compose::Point::new(coord.clone());
        let combo = crate::compose::Combination {
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
                axis_a: 0,
                axis_b: 1,
            }],
            ProjectorSpec::Sum,
        );
        let combos = crate::compose::expand_all(&spec).unwrap();
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
                axis_a: 0,
                axis_b: 1,
            }],
            ProjectorSpec::Sum,
        );
        let combos = crate::compose::expand_all(&spec).unwrap();
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
                ConstraintSpec::Even { axis: 0 },
                ConstraintSpec::Range {
                    axis: 0,
                    min: 0,
                    max: 10,
                },
            ],
            ProjectorSpec::Identity { axis: 0 },
        );
        let combos = crate::compose::expand_all(&spec).unwrap();
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
                axis_a: 0,
                axis_b: 1,
            }],
            ProjectorSpec::Sum,
        );
        let combos = crate::compose::expand_all(&spec).unwrap();
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
                axis_a: 0,
                axis_b: 1,
            }],
            ProjectorSpec::Sum,
        );
        let combos = crate::compose::expand_all(&spec).unwrap();
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
                axis: 0,
                value: 5,
            }],
            ProjectorSpec::Identity { axis: 0 },
        );
        let combos = crate::compose::expand_all(&spec).unwrap();
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
                axis: 0,
                value: 5,
            }],
            ProjectorSpec::Identity { axis: 0 },
        );
        let combos = crate::compose::expand_all(&spec).unwrap();
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
                axis: 0,
                value: 5,
            }],
            ProjectorSpec::Identity { axis: 0 },
        );
        let combos = crate::compose::expand_all(&spec).unwrap();
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
                axis: 0,
                value: 5,
            }],
            ProjectorSpec::Identity { axis: 0 },
        );
        let combos = crate::compose::expand_all(&spec).unwrap();
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
                axis: 0,
                values: vec![0, 2, 4],
            }],
            ProjectorSpec::Identity { axis: 0 },
        );
        let combos = crate::compose::expand_all(&spec).unwrap();
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

    // ── Edge cases ────────────────────────────────────────────────

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
