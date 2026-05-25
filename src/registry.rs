//! Type registries — map named constraint/projector types to builders.
//!
//! Following the Nexus capability-trait pattern, registries decouple the
//! input format (YAML type names) from the concrete implementations. New
//! constraint or projector types register themselves without modifying
//! the pipeline core.

use crate::spec::{ConstraintSpec, ProjectorSpec};
use ssccs_core::{Constraint, Coordinates, Field, Projector, Segment};
use std::collections::HashMap;

// ============================================================================
// Constraint Registry
// ============================================================================

/// Builds a boxed constraint from a specification.
pub type ConstraintBuilder = fn(spec: &ConstraintSpec) -> AnyConstraint;

/// Type-erased constraint wrapper — bridges `Box<dyn Constraint>` into
/// `impl Constraint + 'static` for Field::add_constraint().
#[derive(Debug)]
pub struct AnyConstraint(Box<dyn Constraint>);

impl AnyConstraint {
    pub fn new(c: impl Constraint + 'static) -> Self {
        Self(Box::new(c))
    }
}

impl Constraint for AnyConstraint {
    fn allows(&self, coords: &Coordinates) -> bool {
        self.0.allows(coords)
    }
    fn describe(&self) -> String {
        self.0.describe()
    }
}

/// Registry of named constraint types → builders.
///
/// Use `default_registry()` for the standard set, or build a custom one
/// for domain-specific constraints.
pub struct ConstraintRegistry {
    builders: HashMap<String, ConstraintBuilder>,
}

impl ConstraintRegistry {
    pub fn new() -> Self {
        Self {
            builders: HashMap::new(),
        }
    }

    /// Register a constraint builder under a type name.
    pub fn register(&mut self, type_name: &str, builder: ConstraintBuilder) {
        self.builders.insert(type_name.to_string(), builder);
    }

    /// Build a constraint from a spec. Returns None if the type is unknown.
    pub fn build(&self, spec: &ConstraintSpec) -> Option<AnyConstraint> {
        let type_name = spec_type_name(spec);
        self.builders.get(type_name).map(|b| b(spec))
    }

    /// Build all constraints from a list of specs, skipping unknown types.
    pub fn build_all(&self, specs: &[ConstraintSpec]) -> Vec<AnyConstraint> {
        specs.iter().filter_map(|s| self.build(s)).collect()
    }
}

impl Default for ConstraintRegistry {
    fn default() -> Self {
        let mut reg = Self::new();
        reg.register("range", |spec| {
            if let ConstraintSpec::Range { axis, min, max } = spec {
                AnyConstraint::new(RangeC {
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
                AnyConstraint::new(EvenC { axis: *axis })
            } else {
                panic!("even builder called on non-even spec")
            }
        });
        reg.register("eq", |spec| {
            if let ConstraintSpec::Eq { axis_a, axis_b } = spec {
                AnyConstraint::new(EqC {
                    axis_a: *axis_a,
                    axis_b: *axis_b,
                })
            } else {
                panic!("eq builder called on non-eq spec")
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
    }
}

// ── Built-in constraint implementations ──

#[derive(Debug, Clone)]
struct RangeC {
    axis: usize,
    min: i64,
    max: i64,
}

impl Constraint for RangeC {
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

impl Constraint for EvenC {
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

impl Constraint for EqC {
    fn allows(&self, coords: &Coordinates) -> bool {
        let a = coords.get_axis(self.axis_a);
        let b = coords.get_axis(self.axis_b);
        a.is_some() && b.is_some() && a == b
    }
    fn describe(&self) -> String {
        format!("axis[{}] == axis[{}]", self.axis_a, self.axis_b)
    }
}

// ============================================================================
// Projector Registry
// ============================================================================

/// A type-erased projector that delegates to a concrete implementation.
///
/// ssccs_core::Projector has an associated type (Output), which prevents
/// storing heterogeneous projectors in a Vec. We erase the output type
/// by boxing to i64.
pub trait ErasedProjector: std::fmt::Debug + Send + Sync {
    fn project_i64(&self, field: &Field, segment: &Segment) -> Option<i64>;
}

impl<P: Projector<Output = i64>> ErasedProjector for P {
    fn project_i64(&self, field: &Field, segment: &Segment) -> Option<i64> {
        self.project(field, segment)
    }
}

/// Builds a boxed erased projector from a specification.
pub type ProjectorBuilder = fn(spec: &ProjectorSpec) -> Box<dyn ErasedProjector>;

/// Registry of named projector types → builders.
pub struct ProjectorRegistry {
    builders: HashMap<String, ProjectorBuilder>,
}

impl ProjectorRegistry {
    pub fn new() -> Self {
        Self {
            builders: HashMap::new(),
        }
    }

    pub fn register(&mut self, type_name: &str, builder: ProjectorBuilder) {
        self.builders.insert(type_name.to_string(), builder);
    }

    pub fn build(&self, spec: &ProjectorSpec) -> Option<Box<dyn ErasedProjector>> {
        let type_name = spec_projector_name(spec);
        self.builders.get(type_name).map(|b| b(spec))
    }
}

impl Default for ProjectorRegistry {
    fn default() -> Self {
        let mut reg = Self::new();
        reg.register("sum", |_spec| Box::new(SumP));
        reg.register("identity", |spec| {
            if let ProjectorSpec::Identity { axis } = spec {
                Box::new(IdentityP { axis: *axis })
            } else {
                panic!("identity builder called on non-identity spec")
            }
        });
        reg.register("parity", |spec| {
            if let ProjectorSpec::Parity { axis } = spec {
                Box::new(ParityP { axis: *axis })
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

// ── Built-in projector implementations ──

#[derive(Debug, Clone)]
struct SumP;

impl Projector for SumP {
    type Output = i64;
    fn project(&self, _field: &Field, segment: &Segment) -> Option<Self::Output> {
        Some(segment.coordinates().raw.iter().sum())
    }
}

#[derive(Debug, Clone)]
struct IdentityP {
    axis: usize,
}

impl Projector for IdentityP {
    type Output = i64;
    fn project(&self, _field: &Field, segment: &Segment) -> Option<Self::Output> {
        segment.coordinates().get_axis(self.axis)
    }
}

#[derive(Debug, Clone)]
struct ParityP {
    axis: usize,
}

impl Projector for ParityP {
    type Output = i64;
    fn project(&self, _field: &Field, segment: &Segment) -> Option<Self::Output> {
        segment.coordinates().get_axis(self.axis).map(|v| v & 1)
    }
}
