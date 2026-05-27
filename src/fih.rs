//! Fact primitives — ev's output envelope for neXus consumption.
//!
//! Every output ev produces (verification results, synthesis reports) is
//! wrapped in a Fact. This is the only interface between ev and any
//! downstream colony.

use serde::{Deserialize, Serialize};

/// A validated observation — immutable once committed.
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
}
