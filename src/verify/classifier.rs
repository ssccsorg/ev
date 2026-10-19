//! Classification capability: the seam every engine implements.
//!
//! The built-in structural enumerator is the default. The naive pipeline is
//! the second in-repo implementation, kept so records from two engines can be
//! compared at the same coordinates. An imported engine implements the same
//! trait and emits the same [`Classification`].

use crate::classification::{ClaimClass, Classification, EngineIdentity};
use crate::spec::VerificationSpec;
use crate::verify::compose::{expand_all, raw_total_combinations};
use crate::verify::evaluate::{evaluate_all, evaluate_structural};
use crate::verify::registry::{ConstraintRegistry, ProjectorRegistry};

/// Capability: classify a declared space.
pub trait Classify: Send + Sync {
    /// Stable name of this engine, used for selection and messages.
    fn name(&self) -> &'static str;

    /// Classify the space. The returned record names this engine and its
    /// claim class.
    fn classify(
        &self,
        spec: &VerificationSpec,
        constraints: &ConstraintRegistry,
        projectors: &ProjectorRegistry,
    ) -> Result<Classification, String>;
}

/// The default engine: structural enumeration, O(V) in the valid set.
pub struct StructuralClassifier;

impl Classify for StructuralClassifier {
    fn name(&self) -> &'static str {
        "structural"
    }

    fn classify(
        &self,
        spec: &VerificationSpec,
        constraints: &ConstraintRegistry,
        projectors: &ProjectorRegistry,
    ) -> Result<Classification, String> {
        evaluate_structural(spec, constraints, projectors)
    }
}

/// The reference engine: full cartesian expansion, O(N) in the raw space.
///
/// It emits a verdict for every point, including points a structural
/// constraint rejects, so its record is comparable with the default engine at
/// each coordinate.
pub struct NaiveClassifier;

impl Classify for NaiveClassifier {
    fn name(&self) -> &'static str {
        "naive"
    }

    fn classify(
        &self,
        spec: &VerificationSpec,
        constraints: &ConstraintRegistry,
        projectors: &ProjectorRegistry,
    ) -> Result<Classification, String> {
        let total = raw_total_combinations(spec)?;
        let combinations = expand_all(spec)?;
        let verdicts = evaluate_all(spec, combinations, constraints, projectors);

        Ok(Classification::by(
            EngineIdentity::new("ev/naive", env!("CARGO_PKG_VERSION")),
            ClaimClass::Exhaustive,
            spec.fields.keys().cloned().collect(),
            total,
            verdicts,
        ))
    }
}
