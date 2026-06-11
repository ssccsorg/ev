//! XIF (eXhaustive Instruction Format) — YAML parser implementing FormatCapable.
//!
//! XIF is the industry-standard YAML input format for ev, compatible with
//! existing RISC-V verification workflows (RISCV-CTG, RISCV-DV, RISCV-Config).

use crate::format::FormatCapable;
use crate::spec::{
    ConstraintSpec, EncodingLayout, FieldBitMapping, FieldSpec, ProjectorSpec, VerificationSpec,
};
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

/// Default R-type encoding layout for RISC-V.
fn default_rtype_encoding() -> EncodingLayout {
    let mut field_map = BTreeMap::new();
    field_map.insert("opcode".into(), FieldBitMapping { pos: 0, width: 7 });
    field_map.insert("rd".into(), FieldBitMapping { pos: 7, width: 5 });
    field_map.insert("funct3".into(), FieldBitMapping { pos: 12, width: 3 });
    field_map.insert("rs1".into(), FieldBitMapping { pos: 15, width: 5 });
    field_map.insert("rs2".into(), FieldBitMapping { pos: 20, width: 5 });
    field_map.insert("funct7".into(), FieldBitMapping { pos: 25, width: 7 });
    EncodingLayout {
        insn_width: 32,
        field_map,
    }
}

/// R4 encoding layout (R-type with func2 replacing funct7 bits).
fn default_r4_encoding() -> EncodingLayout {
    let mut field_map = BTreeMap::new();
    field_map.insert("opcode".into(), FieldBitMapping { pos: 0, width: 7 });
    field_map.insert("rd".into(), FieldBitMapping { pos: 7, width: 5 });
    field_map.insert("funct3".into(), FieldBitMapping { pos: 12, width: 3 });
    field_map.insert("rs1".into(), FieldBitMapping { pos: 15, width: 5 });
    field_map.insert("rs2".into(), FieldBitMapping { pos: 20, width: 5 });
    field_map.insert("func2".into(), FieldBitMapping { pos: 25, width: 2 });
    field_map.insert("rs3".into(), FieldBitMapping { pos: 27, width: 5 });
    EncodingLayout {
        insn_width: 32,
        field_map,
    }
}

// ── Raw deserialization structs ──

#[derive(Debug, Deserialize)]
struct RawXif {
    #[serde(default)]
    target: String,
    #[serde(default)]
    fields: BTreeMap<String, RawField>,
    /// Encoding format: "r" (default, R-type), "r4" (R4 with func2), "i" (I-type), etc.
    #[serde(default)]
    encoding: Option<String>,
    /// Optional explicit encoding layout. If absent, derived from `encoding` field.
    #[serde(default)]
    encoding_layout: Option<EncodingLayout>,
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

        // Resolve encoding layout: explicit layout > named format > default R-type.
        let encoding = self
            .encoding_layout
            .or_else(|| match self.encoding.as_deref() {
                Some("r4") => Some(default_r4_encoding()),
                _ => Some(default_rtype_encoding()),
            });

        VerificationSpec {
            target: self.target,
            fields,
            encoding,
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
