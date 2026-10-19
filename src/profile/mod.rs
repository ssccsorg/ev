//! Named word layouts.
//!
//! A profile is the bit layout a target family uses, selected by name in a
//! spec. The registry holds the names and the engine itself names no
//! architecture: a built-in profile is compiled only when its feature is
//! enabled, and a caller can register its own. The default registry is empty
//! in a generic build.

use crate::spec::WordLayout;
use std::collections::BTreeMap;

type Builder = Box<dyn Fn() -> WordLayout + Send + Sync>;

/// Registry of named layouts, resolved when a spec declares a `profile`.
pub struct LayoutRegistry {
    builders: BTreeMap<String, Builder>,
}

impl LayoutRegistry {
    pub fn new() -> Self {
        Self {
            builders: BTreeMap::new(),
        }
    }

    /// Register a layout under `name`. Re-registering replaces the entry.
    pub fn register<F>(&mut self, name: impl Into<String>, build: F)
    where
        F: Fn() -> WordLayout + Send + Sync + 'static,
    {
        self.builders.insert(name.into(), Box::new(build));
    }

    /// Build the layout registered under `name`.
    pub fn resolve(&self, name: &str) -> Option<WordLayout> {
        self.builders.get(name).map(|build| build())
    }

    /// Registered names, sorted.
    pub fn names(&self) -> Vec<&str> {
        self.builders.keys().map(String::as_str).collect()
    }
}

impl Default for LayoutRegistry {
    fn default() -> Self {
        #[cfg_attr(not(feature = "riscv"), allow(unused_mut))]
        let mut registry = Self::new();
        #[cfg(feature = "riscv")]
        riscv::register(&mut registry);
        registry
    }
}

#[cfg(feature = "riscv")]
pub mod riscv;
