//! Evaluation — runs observation on each constraint combination.
//!
//! Uses the Field-based observation pipeline with pluggable constraints
//! and projectors resolved from registries.

use crate::compose::Combination;
use crate::registry::{ConstraintRegistry, ProjectorRegistry};
use crate::spec::VerificationSpec;
use ssccs_core::{Constraint, Field};

/// Result of evaluating a single constraint combination.
#[derive(Debug, Clone)]
pub struct Evaluation {
    /// The combination that was evaluated.
    pub combination: Combination,
    /// Whether the combination passed all constraints.
    pub passed: bool,
    /// Projection value (meaningful only if passed).
    pub projection: Option<i64>,
    /// Human-readable reason for failure (empty if passed).
    pub reason: String,
}

/// Build a Field from the spec's constraints using the registry.
fn build_field(spec: &VerificationSpec, constraint_registry: &ConstraintRegistry) -> Field {
    let mut field = Field::new();

    // Per-axis domain constraints from field definitions.
    for (axis, (_name, field_spec)) in spec.fields.iter().enumerate() {
        let fs = field_spec.clone();
        field.add_constraint(FieldDomainConstraint {
            axis,
            field_spec: fs,
        });
    }

    // Additional named constraints from the spec, resolved via registry.
    for c in constraint_registry.build_all(&spec.constraints) {
        field.add_constraint(c);
    }

    field
}

/// A constraint that checks a value against its field's domain definition.
#[derive(Debug, Clone)]
struct FieldDomainConstraint {
    axis: usize,
    field_spec: crate::spec::FieldSpec,
}

impl ssccs_core::Constraint for FieldDomainConstraint {
    fn allows(&self, coords: &ssccs_core::Coordinates) -> bool {
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

/// Evaluate all combinations against the field using the given registries.
pub fn evaluate_all(
    spec: &VerificationSpec,
    combinations: Vec<Combination>,
    constraint_registry: &ConstraintRegistry,
    projector_registry: &ProjectorRegistry,
) -> Vec<Evaluation> {
    let field = build_field(spec, constraint_registry);
    let projector = projector_registry
        .build(&spec.projector)
        .expect("projector type must be registered");

    combinations
        .into_iter()
        .map(|combination| {
            // Check field domain constraints first.
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

            // Check named constraints.
            if !field.allows(combination.segment.coordinates()) {
                let mut failures = Vec::new();
                for c in constraint_registry.build_all(&spec.constraints) {
                    if !c.allows(combination.segment.coordinates()) {
                        failures.push(c.describe());
                    }
                }
                return Evaluation {
                    combination,
                    passed: false,
                    projection: None,
                    reason: failures.join("; "),
                };
            }

            // Project through the erased projector.
            let projection = projector.project_i64(&field, &combination.segment);

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
