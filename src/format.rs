//! Format capability — parses input files into VerificationSpec.
//!
//! Following the Nexus capability-trait pattern: each format (YAML, JSON, .ss)
//! implements this trait. The pipeline only depends on the trait, not on
//! any specific format.

use crate::spec::VerificationSpec;
use std::path::Path;

/// Capability: parse a file into a verification specification.
pub trait FormatCapable: Send + Sync {
    /// Parse a file path into a VerificationSpec.
    fn parse(&self, path: &Path) -> anyhow::Result<VerificationSpec>;
}
