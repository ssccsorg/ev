//! Type registries — map named constraint/projector types to builders.

use crate::compose::{Coordinates, Point};
use crate::spec::{ConstraintSpec, FieldSpec, ProjectorSpec};
use std::collections::{BTreeMap, HashMap};

// ============================================================================
// Check trait
// ============================================================================

/// A pass/fail rule on a coordinate vector.
pub trait Check: std::fmt::Debug + Send + Sync {
    fn allows(&self, coords: &Coordinates) -> bool;
    fn describe(&self) -> String;
}

/// Builds a boxed check from a specification.
pub type ConstraintBuilder =
    fn(spec: &ConstraintSpec, axis_of: &HashMap<String, usize>) -> AnyCheck;

/// Build an axis index from a field map.
pub fn build_axis_index(fields: &BTreeMap<String, FieldSpec>) -> HashMap<String, usize> {
    fields
        .keys()
        .enumerate()
        .map(|(i, k)| (k.clone(), i))
        .collect()
}

/// Type-erased check wrapper.
#[derive(Debug)]
pub struct AnyCheck(Box<dyn Check>);

impl AnyCheck {
    pub fn new(c: impl Check + 'static) -> Self {
        Self(Box::new(c))
    }

    pub fn into_check(self) -> Box<dyn Check> {
        self.0
    }
}

impl Check for AnyCheck {
    fn allows(&self, coords: &Coordinates) -> bool {
        self.0.allows(coords)
    }
    fn describe(&self) -> String {
        self.0.describe()
    }
}

/// Registry of named constraint types → builders.
pub struct ConstraintRegistry {
    builders: HashMap<String, ConstraintBuilder>,
}

impl ConstraintRegistry {
    pub fn new() -> Self {
        Self {
            builders: HashMap::new(),
        }
    }

    pub fn register(&mut self, type_name: &str, builder: ConstraintBuilder) {
        self.builders.insert(type_name.to_string(), builder);
    }

    pub fn build(
        &self,
        spec: &ConstraintSpec,
        axis_of: &HashMap<String, usize>,
    ) -> Option<AnyCheck> {
        let type_name = spec_type_name(spec);
        self.builders.get(type_name).map(|b| b(spec, axis_of))
    }

    pub fn build_all(
        &self,
        specs: &[ConstraintSpec],
        fields: &BTreeMap<String, FieldSpec>,
    ) -> Vec<AnyCheck> {
        let axis_of = build_axis_index(fields);
        specs
            .iter()
            .filter_map(|s| self.build(s, &axis_of))
            .collect()
    }

}

impl Default for ConstraintRegistry {
    fn default() -> Self {
        let mut reg = Self::new();
        reg.register("range", |spec, axis_of| {
            if let ConstraintSpec::Range { field, min, max } = spec {
                let axis = axis_of[field];
                AnyCheck::new(RangeC {
                    field_name: field.clone(),
                    axis,
                    min: *min,
                    max: *max,
                })
            } else {
                panic!("range builder called on non-range spec")
            }
        });
        reg.register("even", |spec, axis_of| {
            if let ConstraintSpec::Even { field } = spec {
                let axis = axis_of[field];
                AnyCheck::new(EvenC {
                    field_name: field.clone(),
                    axis,
                })
            } else {
                panic!("even builder called on non-even spec")
            }
        });
        reg.register("eq", |spec, axis_of| {
            if let ConstraintSpec::Eq { field_a, field_b } = spec {
                let axis_a = axis_of[field_a];
                let axis_b = axis_of[field_b];
                AnyCheck::new(EqC {
                    field_a: field_a.clone(),
                    axis_a,
                    field_b: field_b.clone(),
                    axis_b,
                })
            } else {
                panic!("eq builder called on non-eq spec")
            }
        });
        reg.register("neq", |spec, axis_of| {
            if let ConstraintSpec::Neq { field_a, field_b } = spec {
                let axis_a = axis_of[field_a];
                let axis_b = axis_of[field_b];
                AnyCheck::new(NeqC {
                    field_a: field_a.clone(),
                    axis_a,
                    field_b: field_b.clone(),
                    axis_b,
                })
            } else {
                panic!("neq builder called on non-neq spec")
            }
        });
        reg.register("lt", |spec, axis_of| {
            if let ConstraintSpec::Lt { field, value } = spec {
                let axis = axis_of[field];
                AnyCheck::new(LtC {
                    field_name: field.clone(),
                    axis,
                    value: *value,
                })
            } else {
                panic!("lt builder called on non-lt spec")
            }
        });
        reg.register("gt", |spec, axis_of| {
            if let ConstraintSpec::Gt { field, value } = spec {
                let axis = axis_of[field];
                AnyCheck::new(GtC {
                    field_name: field.clone(),
                    axis,
                    value: *value,
                })
            } else {
                panic!("gt builder called on non-gt spec")
            }
        });
        reg.register("le", |spec, axis_of| {
            if let ConstraintSpec::Le { field, value } = spec {
                let axis = axis_of[field];
                AnyCheck::new(LeC {
                    field_name: field.clone(),
                    axis,
                    value: *value,
                })
            } else {
                panic!("le builder called on non-le spec")
            }
        });
        reg.register("ge", |spec, axis_of| {
            if let ConstraintSpec::Ge { field, value } = spec {
                let axis = axis_of[field];
                AnyCheck::new(GeC {
                    field_name: field.clone(),
                    axis,
                    value: *value,
                })
            } else {
                panic!("ge builder called on non-ge spec")
            }
        });
        reg.register("oneof", |spec, axis_of| {
            if let ConstraintSpec::Oneof { field, values } = spec {
                let axis = axis_of[field];
                AnyCheck::new(OneofC {
                    field_name: field.clone(),
                    axis,
                    values: values.clone(),
                })
            } else {
                panic!("oneof builder called on non-oneof spec")
            }
        });
        reg.register("cross", |spec, axis_of| {
            if let ConstraintSpec::Cross {
                field_a,
                field_b,
                mapping,
            } = spec
            {
                AnyCheck::new(CrossC {
                    field_a: field_a.clone(),
                    axis_a: axis_of[field_a],
                    field_b: field_b.clone(),
                    axis_b: axis_of[field_b],
                    mapping: mapping.clone(),
                })
            } else {
                panic!("cross builder called on non-cross spec")
            }
        });
        reg
    }
}

fn spec_type_name(spec: &ConstraintSpec) -> &str {
    match spec {
        ConstraintSpec::Range { .. } => "range",
        ConstraintSpec::Even { .. } => "even",
        ConstraintSpec::Eq { .. } => "eq",
        ConstraintSpec::Neq { .. } => "neq",
        ConstraintSpec::Lt { .. } => "lt",
        ConstraintSpec::Gt { .. } => "gt",
        ConstraintSpec::Le { .. } => "le",
        ConstraintSpec::Ge { .. } => "ge",
        ConstraintSpec::Oneof { .. } => "oneof",
        ConstraintSpec::Cross { .. } => "cross",
        ConstraintSpec::EnableMask { .. } => "enable_mask",
    }
}

// ── Built-in check implementations ──

#[derive(Debug, Clone)]
struct RangeC {
    field_name: String,
    axis: usize,
    min: i64,
    max: i64,
}

impl Check for RangeC {
    fn allows(&self, coords: &Coordinates) -> bool {
        coords
            .get_axis(self.axis)
            .map(|v| v >= self.min && v <= self.max)
            .unwrap_or(false)
    }
    fn describe(&self) -> String {
        format!("{} ∈ [{}, {}]", self.field_name, self.min, self.max)
    }
}

#[derive(Debug, Clone)]
struct EvenC {
    field_name: String,
    axis: usize,
}

impl Check for EvenC {
    fn allows(&self, coords: &Coordinates) -> bool {
        coords
            .get_axis(self.axis)
            .map(|v| v % 2 == 0)
            .unwrap_or(false)
    }
    fn describe(&self) -> String {
        format!("{} is even", self.field_name)
    }
}

#[derive(Debug, Clone)]
struct EqC {
    field_a: String,
    axis_a: usize,
    field_b: String,
    axis_b: usize,
}

impl Check for EqC {
    fn allows(&self, coords: &Coordinates) -> bool {
        let a = coords.get_axis(self.axis_a);
        let b = coords.get_axis(self.axis_b);
        a.is_some() && b.is_some() && a == b
    }
    fn describe(&self) -> String {
        format!("{} == {}", self.field_a, self.field_b)
    }
}

#[derive(Debug, Clone)]
struct NeqC {
    field_a: String,
    axis_a: usize,
    field_b: String,
    axis_b: usize,
}

impl Check for NeqC {
    fn allows(&self, coords: &Coordinates) -> bool {
        let a = coords.get_axis(self.axis_a);
        let b = coords.get_axis(self.axis_b);
        a.is_some() && b.is_some() && a != b
    }
    fn describe(&self) -> String {
        format!("{} != {}", self.field_a, self.field_b)
    }
}

#[derive(Debug, Clone)]
struct LtC {
    field_name: String,
    axis: usize,
    value: i64,
}

impl Check for LtC {
    fn allows(&self, coords: &Coordinates) -> bool {
        coords
            .get_axis(self.axis)
            .map(|v| v < self.value)
            .unwrap_or(false)
    }
    fn describe(&self) -> String {
        format!("{} < {}", self.field_name, self.value)
    }
}

#[derive(Debug, Clone)]
struct GtC {
    field_name: String,
    axis: usize,
    value: i64,
}

impl Check for GtC {
    fn allows(&self, coords: &Coordinates) -> bool {
        coords
            .get_axis(self.axis)
            .map(|v| v > self.value)
            .unwrap_or(false)
    }
    fn describe(&self) -> String {
        format!("{} > {}", self.field_name, self.value)
    }
}

#[derive(Debug, Clone)]
struct LeC {
    field_name: String,
    axis: usize,
    value: i64,
}

impl Check for LeC {
    fn allows(&self, coords: &Coordinates) -> bool {
        coords
            .get_axis(self.axis)
            .map(|v| v <= self.value)
            .unwrap_or(false)
    }
    fn describe(&self) -> String {
        format!("{} <= {}", self.field_name, self.value)
    }
}

#[derive(Debug, Clone)]
struct GeC {
    field_name: String,
    axis: usize,
    value: i64,
}

impl Check for GeC {
    fn allows(&self, coords: &Coordinates) -> bool {
        coords
            .get_axis(self.axis)
            .map(|v| v >= self.value)
            .unwrap_or(false)
    }
    fn describe(&self) -> String {
        format!("{} >= {}", self.field_name, self.value)
    }
}

#[derive(Debug, Clone)]
struct OneofC {
    field_name: String,
    axis: usize,
    values: Vec<i64>,
}

impl Check for OneofC {
    fn allows(&self, coords: &Coordinates) -> bool {
        coords
            .get_axis(self.axis)
            .map(|v| self.values.contains(&v))
            .unwrap_or(false)
    }
    fn describe(&self) -> String {
        format!(
            "{} ∈ {{{}}}",
            self.field_name,
            self.values
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

#[derive(Debug, Clone)]
struct CrossC {
    field_a: String,
    axis_a: usize,
    field_b: String,
    axis_b: usize,
    mapping: std::collections::HashMap<i64, Vec<i64>>,
}

impl Check for CrossC {
    fn allows(&self, coords: &Coordinates) -> bool {
        let a = coords.get_axis(self.axis_a);
        let b = coords.get_axis(self.axis_b);
        match (a, b) {
            (Some(va), Some(vb)) => self
                .mapping
                .get(&va)
                .map(|allowed| allowed.contains(&vb))
                .unwrap_or(true),
            _ => false,
        }
    }
    fn describe(&self) -> String {
        let entries: Vec<String> = self
            .mapping
            .iter()
            .map(|(k, v)| {
                format!(
                    "{}={} → [{}]",
                    self.field_a,
                    k,
                    v.iter()
                        .map(|x| x.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })
            .collect();
        format!(
            "{} → {}: {{{}}}",
            self.field_a,
            self.field_b,
            entries.join("; ")
        )
    }
}

// ============================================================================
// Evaluator trait and registry
// ============================================================================

/// Maps a coordinate point to a projected value.
pub trait Evaluator: std::fmt::Debug + Send + Sync {
    fn evaluate(&self, point: &Point) -> Option<i64>;
}

/// Type-erased evaluator.
pub trait ErasedEvaluator: std::fmt::Debug + Send + Sync {
    fn evaluate(&self, point: &Point) -> Option<i64>;
}

impl<E: Evaluator> ErasedEvaluator for E {
    fn evaluate(&self, point: &Point) -> Option<i64> {
        self.evaluate(point)
    }
}

pub type EvaluatorBuilder =
    fn(spec: &ProjectorSpec, axis_of: &HashMap<String, usize>) -> Box<dyn ErasedEvaluator>;

/// Registry of named evaluator types → builders.
pub struct ProjectorRegistry {
    builders: HashMap<String, EvaluatorBuilder>,
}

impl ProjectorRegistry {
    pub fn new() -> Self {
        Self {
            builders: HashMap::new(),
        }
    }

    pub fn register(&mut self, type_name: &str, builder: EvaluatorBuilder) {
        self.builders.insert(type_name.to_string(), builder);
    }

    pub fn build(
        &self,
        spec: &ProjectorSpec,
        axis_of: &HashMap<String, usize>,
    ) -> Option<Box<dyn ErasedEvaluator>> {
        let type_name = spec_projector_name(spec);
        self.builders.get(type_name).map(|b| b(spec, axis_of))
    }

    pub fn resolve(
        &self,
        spec: &ProjectorSpec,
        fields: &BTreeMap<String, FieldSpec>,
    ) -> Option<Box<dyn ErasedEvaluator>> {
        let axis_of = build_axis_index(fields);
        self.build(spec, &axis_of)
    }
}

impl Default for ProjectorRegistry {
    fn default() -> Self {
        let mut reg = Self::new();
        reg.register("sum", |_spec, _axis_of| Box::new(SumEval));
        reg.register("identity", |spec, axis_of| {
            if let ProjectorSpec::Identity { field } = spec {
                let axis = axis_of[field];
                Box::new(IdentityEval { axis })
            } else {
                panic!("identity builder called on non-identity spec")
            }
        });
        reg.register("parity", |spec, axis_of| {
            if let ProjectorSpec::Parity { field } = spec {
                let axis = axis_of[field];
                Box::new(ParityEval { axis })
            } else {
                panic!("parity builder called on non-parity spec")
            }
        });
        reg
    }
}

fn spec_projector_name(spec: &ProjectorSpec) -> &str {
    match spec {
        ProjectorSpec::Sum => "sum",
        ProjectorSpec::Identity { .. } => "identity",
        ProjectorSpec::Parity { .. } => "parity",
    }
}

#[derive(Debug, Clone)]
struct SumEval;

impl Evaluator for SumEval {
    fn evaluate(&self, point: &Point) -> Option<i64> {
        Some(point.coordinates().raw.iter().sum())
    }
}

#[derive(Debug, Clone)]
struct IdentityEval {
    axis: usize,
}

impl Evaluator for IdentityEval {
    fn evaluate(&self, point: &Point) -> Option<i64> {
        point.coordinates().get_axis(self.axis)
    }
}

#[derive(Debug, Clone)]
struct ParityEval {
    axis: usize,
}

impl Evaluator for ParityEval {
    fn evaluate(&self, point: &Point) -> Option<i64> {
        point.coordinates().get_axis(self.axis).map(|v| v & 1)
    }
}
