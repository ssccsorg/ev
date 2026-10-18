//! XIF (eXhaustive Input Format) — YAML parser implementing FormatCapable.
//!
//! The format is target-neutral: fields, constraints, and a projector describe
//! any discrete space. A bit layout is either declared inline under `layout`
//! or selected by name under `profile`, which the registry resolves. A spec
//! that declares neither has no word layout.

use crate::format::FormatCapable;
use crate::profile::LayoutRegistry;
use crate::spec::{ConstraintSpec, FieldSpec, ProjectorSpec, VerificationSpec, WordLayout};
use anyhow::Context;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

/// YAML format parser — implements FormatCapable.
pub struct YamlFormat;

impl FormatCapable for YamlFormat {
    fn parse_with(
        &self,
        path: &Path,
        profiles: &LayoutRegistry,
    ) -> anyhow::Result<VerificationSpec> {
        let content = std::fs::read_to_string(path).context("Failed to read YAML file")?;
        let raw: RawXif = serde_yaml::from_str(&content).context("Failed to parse YAML")?;
        let spec = raw.into_spec(profiles)?;
        spec.projector
            .validate()
            .map_err(|message| anyhow::anyhow!("invalid projector: {message}"))?;
        Ok(spec)
    }
}

// ── Raw deserialization structs ──

#[derive(Debug, Deserialize)]
struct RawXif {
    #[serde(default)]
    target: String,
    #[serde(default)]
    fields: BTreeMap<String, RawField>,
    /// Named layout profile, resolved through the registry. `encoding` is the
    /// key older fixtures use.
    #[serde(default, alias = "encoding")]
    profile: Option<String>,
    /// Explicit bit layout. Mutually exclusive with `profile`.
    #[serde(default, alias = "encoding_layout")]
    layout: Option<WordLayout>,
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
    fn into_spec(self, profiles: &LayoutRegistry) -> anyhow::Result<VerificationSpec> {
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

        let layout = match (self.layout, self.profile) {
            (Some(_), Some(_)) => {
                anyhow::bail!("declare either `layout` or `profile`, not both")
            }
            (Some(layout), None) => Some(layout),
            (None, Some(name)) => Some(resolve_profile(profiles, &name)?),
            (None, None) => None,
        };

        Ok(VerificationSpec {
            target: self.target,
            fields,
            layout,
            constraints: self.constraints,
            projector: self.projector,
        })
    }
}

fn resolve_profile(profiles: &LayoutRegistry, name: &str) -> anyhow::Result<WordLayout> {
    profiles.resolve(name).ok_or_else(|| {
        let known = profiles.names();
        if known.is_empty() {
            anyhow::anyhow!("unknown profile `{name}`: this build has no layout profiles")
        } else {
            anyhow::anyhow!(
                "unknown profile `{name}`; known profiles: {}",
                known.join(", ")
            )
        }
    })
}

/// Convenience: parse a YAML file directly (without going through the trait).
impl VerificationSpec {
    pub fn from_yaml(path: &Path) -> anyhow::Result<Self> {
        YamlFormat.parse(path)
    }

    /// Parse with a caller-supplied layout registry.
    pub fn from_yaml_with(path: &Path, profiles: &LayoutRegistry) -> anyhow::Result<Self> {
        YamlFormat.parse_with(path, profiles)
    }
}
