//! Format capability — parses input files into VerificationSpec.
//!
//! Each format (YAML, JSON, .ss) implements this trait. The pipeline only
//! depends on the trait, not on any specific format. Parsing takes the layout
//! registry so a caller can supply profiles the built-ins do not cover.

use crate::profile::LayoutRegistry;
use crate::spec::VerificationSpec;
use std::path::Path;

/// Capability: parse a file into a verification specification.
pub trait FormatCapable: Send + Sync {
    /// Parse with an explicit layout registry.
    fn parse_with(
        &self,
        path: &Path,
        profiles: &LayoutRegistry,
    ) -> anyhow::Result<VerificationSpec>;

    /// Parse with the built-in profiles.
    fn parse(&self, path: &Path) -> anyhow::Result<VerificationSpec> {
        self.parse_with(path, &LayoutRegistry::default())
    }
}

pub mod xif;
