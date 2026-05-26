//! Fact–Intent–Hint primitives — the universal interface between ev and neXus.
//!
//! Every output ev produces is a Fact. Every input it receives is an Intent.
//! Constraints extracted from failures become Hints. These three primitives
//! form the only interface between any two colonies in the SSCCS ecosystem.

use serde::{Deserialize, Serialize};

/// A validated observation — immutable once committed.
///
/// Facts are ev's primary output. Every verification result and every
/// synthesis report is a Fact. Once written to the shared surface, a Fact
/// cannot be retracted or modified.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fact {
    /// Stable type tag: "verification_result", "synthesis_result", etc.
    pub fact_type: String,
    /// Origin identifier: "ev/0.1.0", "ev/synthesis/yosys".
    pub origin: String,
    /// Target module or instruction identifier.
    pub target: String,
    /// Type-specific payload — the actual data.
    pub payload: serde_json::Value,
    /// ISO 8601 timestamp of observation.
    pub timestamp: String,
    /// Optional hash of the parent Fact that triggered this observation.
    pub parent_fact_id: Option<String>,
}

/// A proposed exploration — an action to be claimed and executed.
///
/// ev receives Intents from neXus (e.g., "verify this XIF spec") and
/// produces Facts in response. ev does not generate Intents itself in
/// the current phase.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct Intent {
    pub action: String,
    pub parameters: serde_json::Value,
    pub confidence: f64,
    pub parent_fact_id: Option<String>,
}

/// An injected constraint — narrows the space of admissible operations.
///
/// Constraints from the spec become Hints on the shared surface. neXus
/// detectors consume Hints to understand what boundaries an entity
/// operates within.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct Hint {
    pub hint_type: String,
    pub content: String,
    pub scope: String,
}

impl Fact {
    /// Create a Fact with the current UTC timestamp.
    pub fn new(
        fact_type: impl Into<String>,
        origin: impl Into<String>,
        target: impl Into<String>,
        payload: serde_json::Value,
    ) -> Self {
        let timestamp = chrono::Utc::now().to_rfc3339();
        Self {
            fact_type: fact_type.into(),
            origin: origin.into(),
            target: target.into(),
            payload,
            timestamp,
            parent_fact_id: None,
        }
    }

    /// Chain this Fact to a parent.
    #[allow(dead_code)]
    pub fn with_parent(mut self, parent_id: impl Into<String>) -> Self {
        self.parent_fact_id = Some(parent_id.into());
        self
    }
}
