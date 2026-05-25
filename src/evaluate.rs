//! Evaluation — runs observation on each constraint combination.
//!
//! Uses the Field-based observation pipeline: builds a Field with per-axis
//! constraints, then calls `observe()` for each combination.

use crate::compose::Combination;
use crate::xif::{FieldDef, XifDocument};
use ssccs_core::{observe, Coordinates, Field, Projector, Segment};

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

/// A projector that sums all coordinate axes.
#[derive(Debug, Clone)]
pub struct SumProjector;

impl Projector for SumProjector {
    type Output = i64;

    fn project(&self, _field: &Field, segment: &Segment) -> Option<Self::Output> {
        let coords = segment.coordinates();
        let sum: i64 = coords.raw.iter().sum();
        Some(sum)
    }
}

/// Build a Field from the XIF field definitions.
///
/// Each field becomes an axis constraint: the value must satisfy the field's
/// domain definition.
fn build_field(doc: &XifDocument) -> Field {
    let mut field = Field::new();
    for (axis, (_name, def)) in doc.fields.iter().enumerate() {
        match def {
            FieldDef::Range { range, alignment } => {
                let r0 = range[0];
                let r1 = range[1];
                let align = alignment.unwrap_or(1);
                field.add_constraint(AxisConstraint {
                    axis,
                    allowed: move |v: i64| v >= r0 && v <= r1 && v % align == 0,
                    desc: format!("axis[{}] ∈ [{}, {}] step {}", axis, r0, r1, align),
                });
            }
            FieldDef::Values { values } => {
                let allowed_vals: Vec<i64> = values
                    .iter()
                    .filter_map(|v| match v {
                        serde_yaml::Value::Number(n) => n.as_i64(),
                        serde_yaml::Value::Bool(b) => Some(if *b { 1 } else { 0 }),
                        _ => None,
                    })
                    .collect();
                let desc = format!(
                    "axis[{}] ∈ {:?}",
                    axis,
                    allowed_vals
                        .iter()
                        .map(|v| v.to_string())
                        .collect::<Vec<_>>()
                );
                field.add_constraint(AxisConstraint {
                    axis,
                    allowed: {
                        let vals = allowed_vals;
                        move |v: i64| vals.contains(&v)
                    },
                    desc,
                });
            }
        }
    }
    field
}

/// A per-axis constraint adapter.
#[derive(Clone)]
struct AxisConstraint<F> {
    axis: usize,
    allowed: F,
    desc: String,
}

impl<F> std::fmt::Debug for AxisConstraint<F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AxisConstraint")
            .field("axis", &self.axis)
            .field("desc", &self.desc)
            .finish()
    }
}

impl<F: Fn(i64) -> bool + Send + Sync + 'static> ssccs_core::Constraint for AxisConstraint<F> {
    fn allows(&self, coords: &Coordinates) -> bool {
        coords
            .get_axis(self.axis)
            .map(|v| (self.allowed)(v))
            .unwrap_or(false)
    }

    fn describe(&self) -> String {
        self.desc.clone()
    }
}

/// Evaluate all combinations against the field.
pub fn evaluate_all(doc: &XifDocument, combinations: Vec<Combination>) -> Vec<Evaluation> {
    let field = build_field(doc);
    let projector = SumProjector;

    combinations
        .into_iter()
        .map(|combination| {
            let result = observe(&field, &combination.segment, &projector);
            match result {
                Some(projection) => Evaluation {
                    combination,
                    passed: true,
                    projection: Some(projection),
                    reason: String::new(),
                },
                None => {
                    // Determine which constraints failed.
                    let mut failures = Vec::new();
                    for (axis, (name, def)) in doc.fields.iter().enumerate() {
                        if let Some(value) = combination.coordinates.get_axis(axis) {
                            if !def.allows(value) {
                                failures.push(format!(
                                    "{}={} (expected {})",
                                    name,
                                    value,
                                    def.describe()
                                ));
                            }
                        }
                    }
                    Evaluation {
                        combination,
                        passed: false,
                        projection: None,
                        reason: failures.join("; "),
                    }
                }
            }
        })
        .collect()
}

/// Extension trait for FieldDef to provide a human-readable description.
impl FieldDef {
    fn describe(&self) -> String {
        match self {
            FieldDef::Range { range, alignment } => {
                let align = alignment.unwrap_or(1);
                if align == 1 {
                    format!("{}..={}", range[0], range[1])
                } else {
                    format!("{}..={} step {}", range[0], range[1], align)
                }
            }
            FieldDef::Values { values } => {
                let vals: Vec<String> = values
                    .iter()
                    .map(|v| match v {
                        serde_yaml::Value::Number(n) => n.to_string(),
                        serde_yaml::Value::Bool(b) => b.to_string(),
                        _ => "?".to_string(),
                    })
                    .collect();
                format!("{{{}}}", vals.join(", "))
            }
        }
    }
}
