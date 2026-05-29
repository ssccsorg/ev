//! Verification specification — the internal representation that all input
//! formats parse into. Format-agnostic and constraint-type-agnostic.

use serde::Deserialize;
use std::collections::BTreeMap;

/// Unified internal representation of a verification target.
///
/// All input formats (YAML, JSON, .ss) parse into this struct. The pipeline
/// then resolves named constraint and projector types via registries.
#[derive(Debug, Clone)]
pub struct VerificationSpec {
    /// Target name (instruction, accelerator, or module identifier).
    pub target: String,
    /// Ordered field definitions (deterministic: BTreeMap iteration order).
    pub fields: BTreeMap<String, FieldSpec>,
    /// Named constraint specifications to resolve via ConstraintRegistry.
    pub constraints: Vec<ConstraintSpec>,
    /// Named projector specification to resolve via ProjectorRegistry.
    pub projector: ProjectorSpec,
}

/// Specification for a single field's domain.
#[derive(Debug, Clone)]
pub struct FieldSpec {
    /// If present, the field must be in this range.
    pub range: Option<(i64, i64)>,
    /// If present, the field must be a multiple of this value.
    pub alignment: Option<i64>,
    /// If present, the field must be one of these explicit values.
    pub values: Option<Vec<i64>>,
}

impl FieldSpec {
    /// All values this field can take (domain expansion).
    pub fn expand(&self) -> Vec<i64> {
        if let Some(ref values) = self.values {
            return values.clone();
        }
        let (min, max) = self.range.unwrap_or((0, 255));
        let step = self.alignment.unwrap_or(1);
        (min..=max).filter(|v| v % step == 0).collect()
    }

    /// Check whether a value satisfies this field.
    pub fn allows(&self, value: i64) -> bool {
        if let Some(ref values) = self.values {
            return values.contains(&value);
        }
        if let Some((min, max)) = self.range {
            if value < min || value > max {
                return false;
            }
        }
        if let Some(align) = self.alignment {
            if value % align != 0 {
                return false;
            }
        }
        true
    }
}

/// A constraint to resolve from the ConstraintRegistry.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum ConstraintSpec {
    /// Axis value must be within [min, max].
    #[serde(rename = "range")]
    Range { field: String, min: i64, max: i64 },
    /// Axis value must be even.
    #[serde(rename = "even")]
    Even { field: String },
    /// Two axis values must be equal.
    #[serde(rename = "eq")]
    Eq { field_a: String, field_b: String },
    /// Two axis values must not be equal.
    #[serde(rename = "neq")]
    Neq { field_a: String, field_b: String },
    /// Axis value must be less than a constant.
    #[serde(rename = "lt")]
    Lt { field: String, value: i64 },
    /// Axis value must be greater than a constant.
    #[serde(rename = "gt")]
    Gt { field: String, value: i64 },
    /// Axis value must be less than or equal to a constant.
    #[serde(rename = "le")]
    Le { field: String, value: i64 },
    /// Axis value must be greater than or equal to a constant.
    #[serde(rename = "ge")]
    Ge { field: String, value: i64 },
    /// Axis value must be one of the listed values.
    #[serde(rename = "oneof")]
    Oneof { field: String, values: Vec<i64> },
}

/// A projector to resolve from the ProjectorRegistry.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum ProjectorSpec {
    /// Sum all axis values.
    #[serde(rename = "sum")]
    Sum,
    /// Extract a single axis value.
    #[serde(rename = "identity")]
    Identity {
        field: String,
    },
    /// Classify parity of a single axis.
    #[serde(rename = "parity")]
    Parity {
        field: String,
    },
}
