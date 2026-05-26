//! Evaluation — runs observation on each constraint combination.
//!
//! Uses pluggable checks resolved from registries.

use crate::compose::{Combination, Coordinates};
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

    for (axis, (_name, field_spec)) in spec.fields.iter().enumerate() {
        let fs = field_spec.clone();
        checks.push(Box::new(FieldDomainCheck {
            axis,
            field_spec: fs,
        }));
    }

    for c in registry.build_all(&spec.constraints) {
        checks.push(c.into_check());
    }

    checks
}

/// A check that validates a coordinate against a field's domain definition.
#[derive(Debug, Clone)]
struct FieldDomainCheck {
    axis: usize,
    field_spec: crate::spec::FieldSpec,
}

impl Check for FieldDomainCheck {
    fn allows(&self, coords: &Coordinates) -> bool {
        coords
            .get_axis(self.axis)
            .map(|v| self.field_spec.allows(v))
            .unwrap_or(false)
    }

    fn describe(&self) -> String {
        if let Some(ref vals) = self.field_spec.values {
            format!(
                "axis[{}] ∈ {{{}}}",
                self.axis,
                vals.iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        } else if let Some((min, max)) = self.field_spec.range {
            let step = self.field_spec.alignment.unwrap_or(1);
            format!("axis[{}] ∈ [{}, {}] step {}", self.axis, min, max, step)
        } else {
            format!("axis[{}] (unconstrained)", self.axis)
        }
    }
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
            let mut field_failures = Vec::new();
            for (axis, (name, field_spec)) in spec.fields.iter().enumerate() {
                if let Some(value) = combination.coordinates.get_axis(axis) {
                    if !field_spec.allows(value) {
                        field_failures.push(format!(
                            "{}={} (expected {})",
                            name,
                            value,
                            describe_field(field_spec)
                        ));
                    }
                }
            }

            if !field_failures.is_empty() {
                return Evaluation {
                    combination,
                    passed: false,
                    projection: None,
                    reason: field_failures.join("; "),
                };
            }

            for check in &checks {
                if !check.allows(combination.point.coordinates()) {
                    let mut failures: Vec<String> = Vec::new();
                    for c in constraint_registry.build_all(&spec.constraints) {
                        if !c.allows(combination.point.coordinates()) {
                            failures.push(c.describe());
                        }
                    }
                    return Evaluation {
                        combination,
                        passed: false,
                        projection: None,
                        reason: if failures.is_empty() {
                            check.describe()
                        } else {
                            failures.join("; ")
                        },
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
