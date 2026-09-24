//! The classification of a declared space — the engine's output type.
//!
//! `verify` produces it, and everything downstream of the engine reads it: the
//! reporters, the simulation backends, and any consumer of the library. It
//! carries the field order and the raw total alongside the verdicts, so a
//! reader can interpret a point and reconcile the counts without holding the
//! engine's internal representation (`Combination`, `Coordinates`, `Point`),
//! which stays inside `verify`.
//!
//! The record's own schema — what a Fact payload contains, and how a point is
//! addressed — is not settled here; see issue #66.

/// The verdict on one point of the declared space.
#[derive(Debug, Clone)]
pub struct Verdict {
    /// The point's field values, in the order given by
    /// [`Classification::field_order`].
    pub values: Vec<i64>,
    /// Whether the point satisfies the spec.
    pub passed: bool,
    /// The projector's value for the point, when it produces one.
    pub projection: Option<i64>,
    /// Why the point was rejected. Empty when it passed.
    pub reason: String,
}

/// The classification of one declared space.
#[derive(Debug, Clone)]
pub struct Classification {
    /// Ordered field names matching [`Verdict::values`].
    pub field_order: Vec<String>,
    /// Size of the raw cartesian product of the field domains, computed from
    /// the domains without enumerating the space. A verdict list may be
    /// shorter: the structural pipeline emits only the structurally valid
    /// points, and the counts are derived against this total.
    pub total: usize,
    /// One verdict per point the engine emitted.
    pub verdicts: Vec<Verdict>,
}

impl Classification {
    /// Assemble a classification from its parts.
    pub fn new(field_order: Vec<String>, total: usize, verdicts: Vec<Verdict>) -> Self {
        Self {
            field_order,
            total,
            verdicts,
        }
    }

    /// Passing and failing counts against the raw total.
    ///
    /// Failing is `total - passed` rather than a count over `verdicts`, so the
    /// counts stay authoritative when the verdict list holds only the
    /// structurally valid subset. The invariant is enforced loudly: a silent
    /// `usize` wrap here would corrupt every reported count.
    pub fn counts(&self) -> (usize, usize) {
        let passed = self.verdicts.iter().filter(|v| v.passed).count();
        assert!(
            passed <= self.total,
            "classification invariant violated: passed ({passed}) exceeds raw total ({})",
            self.total
        );
        (passed, self.total - passed)
    }

    /// Points that satisfy the spec, against the raw total.
    pub fn passed(&self) -> usize {
        self.counts().0
    }

    /// Points that do not satisfy the spec, against the raw total.
    pub fn failed(&self) -> usize {
        self.counts().1
    }
}
