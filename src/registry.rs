//! Type registries — map named constraint/projector types to builders.

use crate::compose::{Coordinates, Point};
use crate::spec::{ConstraintSpec, ProjectorSpec};
use std::collections::HashMap;

// ============================================================================
// Check trait
// ============================================================================

/// A pass/fail rule on a coordinate vector.
pub trait Check: std::fmt::Debug + Send + Sync {
    fn allows(&self, coords: &Coordinates) -> bool;
    fn describe(&self) -> String;
}

/// Builds a boxed check from a specification.
pub type ConstraintBuilder = fn(spec: &ConstraintSpec) -> AnyCheck;

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

    pub fn build(&self, spec: &ConstraintSpec) -> Option<AnyCheck> {
        let type_name = spec_type_name(spec);
        self.builders.get(type_name).map(|b| b(spec))
    }

    pub fn build_all(&self, specs: &[ConstraintSpec]) -> Vec<AnyCheck> {
        specs.iter().filter_map(|s| self.build(s)).collect()
    }
}

impl Default for ConstraintRegistry {
    fn default() -> Self {
        let mut reg = Self::new();
        reg.register("range", |spec| {
            if let ConstraintSpec::Range { axis, min, max } = spec {
                AnyCheck::new(RangeC {
                    axis: *axis,
                    min: *min,
                    max: *max,
                })
            } else {
                panic!("range builder called on non-range spec")
            }
        });
        reg.register("even", |spec| {
            if let ConstraintSpec::Even { axis } = spec {
                AnyCheck::new(EvenC { axis: *axis })
            } else {
                panic!("even builder called on non-even spec")
            }
        });
        reg.register("eq", |spec| {
            if let ConstraintSpec::Eq { axis_a, axis_b } = spec {
                AnyCheck::new(EqC {
                    axis_a: *axis_a,
                    axis_b: *axis_b,
                })
            } else {
                panic!("eq builder called on non-eq spec")
            }
        });
        reg.register("neq", |spec| {
            if let ConstraintSpec::Neq { axis_a, axis_b } = spec {
                AnyCheck::new(NeqC {
                    axis_a: *axis_a,
                    axis_b: *axis_b,
                })
            } else {
                panic!("neq builder called on non-neq spec")
            }
        });
        reg.register("lt", |spec| {
            if let ConstraintSpec::Lt { axis, value } = spec {
                AnyCheck::new(LtC {
                    axis: *axis,
                    value: *value,
                })
            } else {
                panic!("lt builder called on non-lt spec")
            }
        });
        reg.register("gt", |spec| {
            if let ConstraintSpec::Gt { axis, value } = spec {
                AnyCheck::new(GtC {
                    axis: *axis,
                    value: *value,
                })
            } else {
                panic!("gt builder called on non-gt spec")
            }
        });
        reg.register("le", |spec| {
            if let ConstraintSpec::Le { axis, value } = spec {
                AnyCheck::new(LeC {
                    axis: *axis,
                    value: *value,
                })
            } else {
                panic!("le builder called on non-le spec")
            }
        });
        reg.register("ge", |spec| {
            if let ConstraintSpec::Ge { axis, value } = spec {
                AnyCheck::new(GeC {
                    axis: *axis,
                    value: *value,
                })
            } else {
                panic!("ge builder called on non-ge spec")
            }
        });
        reg.register("oneof", |spec| {
            if let ConstraintSpec::Oneof { axis, values } = spec {
                AnyCheck::new(OneofC {
                    axis: *axis,
                    values: values.clone(),
                })
            } else {
                panic!("oneof builder called on non-oneof spec")
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
    }
}

// ── Built-in check implementations ──

#[derive(Debug, Clone)]
struct RangeC {
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
        format!("axis[{}] ∈ [{}, {}]", self.axis, self.min, self.max)
    }
}

#[derive(Debug, Clone)]
struct EvenC {
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
        format!("axis[{}] is even", self.axis)
    }
}

#[derive(Debug, Clone)]
struct EqC {
    axis_a: usize,
    axis_b: usize,
}

impl Check for EqC {
    fn allows(&self, coords: &Coordinates) -> bool {
        let a = coords.get_axis(self.axis_a);
        let b = coords.get_axis(self.axis_b);
        a.is_some() && b.is_some() && a == b
    }
    fn describe(&self) -> String {
        format!("axis[{}] == axis[{}]", self.axis_a, self.axis_b)
    }
}

#[derive(Debug, Clone)]
struct NeqC {
    axis_a: usize,
    axis_b: usize,
}

impl Check for NeqC {
    fn allows(&self, coords: &Coordinates) -> bool {
        let a = coords.get_axis(self.axis_a);
        let b = coords.get_axis(self.axis_b);
        a.is_some() && b.is_some() && a != b
    }
    fn describe(&self) -> String {
        format!("axis[{}] != axis[{}]", self.axis_a, self.axis_b)
    }
}

#[derive(Debug, Clone)]
struct LtC {
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
        format!("axis[{}] < {}", self.axis, self.value)
    }
}

#[derive(Debug, Clone)]
struct GtC {
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
        format!("axis[{}] > {}", self.axis, self.value)
    }
}

#[derive(Debug, Clone)]
struct LeC {
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
        format!("axis[{}] <= {}", self.axis, self.value)
    }
}

#[derive(Debug, Clone)]
struct GeC {
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
        format!("axis[{}] >= {}", self.axis, self.value)
    }
}

#[derive(Debug, Clone)]
struct OneofC {
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
            "axis[{}] ∈ {{{}}}",
            self.axis,
            self.values
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(", ")
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

pub type EvaluatorBuilder = fn(spec: &ProjectorSpec) -> Box<dyn ErasedEvaluator>;

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

    pub fn build(&self, spec: &ProjectorSpec) -> Option<Box<dyn ErasedEvaluator>> {
        let type_name = spec_projector_name(spec);
        self.builders.get(type_name).map(|b| b(spec))
    }
}

impl Default for ProjectorRegistry {
    fn default() -> Self {
        let mut reg = Self::new();
        reg.register("sum", |_spec| Box::new(SumEval));
        reg.register("identity", |spec| {
            if let ProjectorSpec::Identity { axis } = spec {
                Box::new(IdentityEval { axis: *axis })
            } else {
                panic!("identity builder called on non-identity spec")
            }
        });
        reg.register("parity", |spec| {
            if let ProjectorSpec::Parity { axis } = spec {
                Box::new(ParityEval { axis: *axis })
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
