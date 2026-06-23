//! XIF (eXhaustive Instruction Format) — YAML parser implementing FormatCapable.
//!
//! XIF is the industry-standard YAML input format for ev, compatible with
//! existing RISC-V verification workflows (RISCV-CTG, RISCV-DV, RISCV-Config).

use crate::format::FormatCapable;
use crate::spec::{ConstraintSpec, FieldSpec, ProjectorSpec, VerificationSpec};
use anyhow::Context;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

/// YAML format parser — implements FormatCapable.
pub struct YamlFormat;

impl FormatCapable for YamlFormat {
    fn parse(&self, path: &Path) -> anyhow::Result<VerificationSpec> {
        let content = std::fs::read_to_string(path).context("Failed to read YAML file")?;
        let raw: RawXif = serde_yaml::from_str(&content).context("Failed to parse YAML")?;
        Ok(raw.into_spec())
    }
}

// ── Raw deserialization structs ──

#[derive(Debug, Deserialize)]
struct RawXif {
    #[serde(default)]
    target: String,
    #[serde(default)]
    fields: BTreeMap<String, RawField>,
    #[serde(default)]
    constraints: Vec<ConstraintSpec>,
    #[serde(default = "default_projector")]
    projector: ProjectorSpec,
}

fn default_projector() -> ProjectorSpec {
    ProjectorSpec::Sum
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RawField {
    Range {
        range: [i64; 2],
        #[serde(default)]
        alignment: Option<i64>,
    },
    Values {
        values: Vec<serde_yaml::Value>,
    },
}

impl RawXif {
    fn into_spec(self) -> VerificationSpec {
        let fields: BTreeMap<String, FieldSpec> = self
            .fields
            .into_iter()
            .map(|(name, raw)| {
                let spec = match raw {
                    RawField::Range { range, alignment } => FieldSpec {
                        range: Some((range[0], range[1])),
                        alignment,
                        values: None,
                    },
                    RawField::Values { values } => {
                        let vals: Vec<i64> = values
                            .iter()
                            .filter_map(|v| match v {
                                serde_yaml::Value::Number(n) => n.as_i64(),
                                serde_yaml::Value::Bool(b) => Some(if *b { 1 } else { 0 }),
                                _ => None,
                            })
                            .collect();
                        FieldSpec {
                            range: None,
                            alignment: None,
                            values: Some(vals),
                        }
                    }
                };
                (name, spec)
            })
            .collect();

        VerificationSpec {
            target: self.target,
            fields,
            encoding: None,
            constraints: self.constraints,
            projector: self.projector,
        }
    }
}

/// Convenience: parse a YAML file directly (without going through the trait).
impl VerificationSpec {
    pub fn from_yaml(path: &Path) -> anyhow::Result<Self> {
        YamlFormat.parse(path)
    }
}
