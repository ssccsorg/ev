//! XIF (eXhaustive Instruction Format) — YAML schema and parser.
//!
//! XIF is the industry-standard YAML input format for ev, compatible with
//! existing RISC-V verification workflows (RISCV-CTG, RISCV-DV, RISCV-Config).
//! It describes instruction fields, their domains, and optional cross-field
//! constraints for exhaustive verification.

use anyhow::Context;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

/// Top-level XIF document.
#[derive(Debug, Deserialize)]
pub struct XifDocument {
    /// Target instruction or accelerator name.
    #[serde(default)]
    pub target: String,
    /// Field definitions — each field maps to one dimension.
    pub fields: BTreeMap<String, FieldDef>,
}

/// Definition of a single instruction field.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum FieldDef {
    /// Contiguous integer range.
    Range {
        range: [i64; 2],
        #[serde(default)]
        alignment: Option<i64>,
    },
    /// Explicit discrete values (integers or booleans).
    Values { values: Vec<serde_yaml::Value> },
}

impl FieldDef {
    /// Expand this field definition into all possible integer values.
    pub fn expand(&self) -> Vec<i64> {
        match self {
            FieldDef::Range { range, alignment } => {
                let align = alignment.unwrap_or(1);
                let start = range[0];
                let end = range[1];
                (start..=end).filter(|v| v % align == 0).collect()
            }
            FieldDef::Values { values } => values
                .iter()
                .filter_map(|v| match v {
                    serde_yaml::Value::Number(n) => n.as_i64(),
                    serde_yaml::Value::Bool(b) => Some(if *b { 1 } else { 0 }),
                    _ => None,
                })
                .collect(),
        }
    }

    /// Check whether a value satisfies this field definition.
    pub fn allows(&self, value: i64) -> bool {
        match self {
            FieldDef::Range { range, alignment } => {
                let align = alignment.unwrap_or(1);
                value >= range[0] && value <= range[1] && value % align == 0
            }
            FieldDef::Values { values } => values.iter().any(|v| match v {
                serde_yaml::Value::Number(n) => n.as_i64() == Some(value),
                serde_yaml::Value::Bool(b) => (*b && value == 1) || (!*b && value == 0),
                _ => false,
            }),
        }
    }
}

impl XifDocument {
    /// Parse a XIF document from a YAML file path.
    pub fn from_path(path: &Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path).context("Failed to read YAML file")?;
        let doc: XifDocument = serde_yaml::from_str(&content).context("Failed to parse YAML")?;
        Ok(doc)
    }

    /// Ordered list of field names (sorted for determinism).
    pub fn field_names(&self) -> Vec<&str> {
        self.fields.keys().map(|s| s.as_str()).collect()
    }
}
