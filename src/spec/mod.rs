//! Verification specification — the internal representation that all input
//! formats parse into. Format-agnostic and constraint-type-agnostic.

use serde::Deserialize;
use std::collections::BTreeMap;

/// Describes how instruction fields map to bits in the encoded instruction word.
///
/// Each entry maps a field name to its bit position (0 = LSB) and width.
/// This is format-agnostic: R-type, R4, I-type, S-type, etc. are all
/// just different field-to-bit mappings.
#[derive(Debug, Clone, Deserialize)]
pub struct EncodingLayout {
    /// Total width in bits (typically 32 for RISC-V).
    #[serde(default = "default_insn_width")]
    pub insn_width: u32,
    /// Field-to-bit mapping: field_name -> (bit_position, bit_width).
    #[serde(default)]
    pub field_map: BTreeMap<String, FieldBitMapping>,
}

fn default_insn_width() -> u32 {
    32
}

/// Bit position and width for a single field in the instruction word.
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct FieldBitMapping {
    /// Bit position (0 = LSB).
    #[serde(default)]
    pub pos: u32,
    /// Bit width.
    pub width: u32,
}

/// Unified internal representation of a verification target.
///
/// All input formats (YAML, JSON, .ss) parse into this struct. The pipeline
/// then resolves named constraint and projector types via registries.
#[derive(Debug, Clone)]
pub struct VerificationSpec {
    /// Target name (instruction, accelerator, or module identifier).
    pub target: String,
    /// Ordered field definitions (deterministic: BTreeMap iteration order).
    pub fields: BTreeMap<String, FieldSpec>,
    /// Instruction word encoding layout (field -> bit position/width).
    pub encoding: Option<EncodingLayout>,
    /// Named constraint specifications to resolve via ConstraintRegistry.
    pub constraints: Vec<ConstraintSpec>,
    /// Named projector specification to resolve via ProjectorRegistry.
    pub projector: ProjectorSpec,
}

/// Specification for a single field's domain.
#[derive(Debug, Clone)]
pub struct FieldSpec {
    /// If present, the field must be in this range.
    pub range: Option<(i64, i64)>,
    /// If present, the field must be a multiple of this value.
    pub alignment: Option<i64>,
    /// If present, the field must be one of these explicit values.
    pub values: Option<Vec<i64>>,
}

impl FieldSpec {
    /// All values this field can take (domain expansion).
    pub fn expand(&self) -> Vec<i64> {
        if let Some(ref values) = self.values {
            return values.clone();
        }
        let (min, max) = self.range.unwrap_or((0, 255));
        let step = self.alignment.unwrap_or(1);
        (min..=max).filter(|v| v % step == 0).collect()
    }

    /// Check whether a value satisfies this field.
    pub fn allows(&self, value: i64) -> bool {
        if let Some(ref values) = self.values {
            return values.contains(&value);
        }
        if let Some((min, max)) = self.range {
            if value < min || value > max {
                return false;
            }
        }
        if let Some(align) = self.alignment {
            if value % align != 0 {
                return false;
            }
        }
        true
    }
}

/// A constraint to resolve from the ConstraintRegistry.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum ConstraintSpec {
    /// Axis value must be within [min, max].
    #[serde(rename = "range")]
    Range { field: String, min: i64, max: i64 },
    /// Axis value must be even.
    #[serde(rename = "even")]
    Even { field: String },
    /// Two axis values must be equal.
    #[serde(rename = "eq")]
    Eq { field_a: String, field_b: String },
    /// Two axis values must not be equal.
    #[serde(rename = "neq")]
    Neq { field_a: String, field_b: String },
    /// Axis value must be less than a constant.
    #[serde(rename = "lt")]
    Lt { field: String, value: i64 },
    /// Axis value must be greater than a constant.
    #[serde(rename = "gt")]
    Gt { field: String, value: i64 },
    /// Axis value must be less than or equal to a constant.
    #[serde(rename = "le")]
    Le { field: String, value: i64 },
    /// Axis value must be greater than or equal to a constant.
    #[serde(rename = "ge")]
    Ge { field: String, value: i64 },
    /// Axis value must be one of the listed values.
    #[serde(rename = "oneof")]
    Oneof { field: String, values: Vec<i64> },
    /// Map field_a values to allowed field_b value sets.
    /// If field_a's value is not in the mapping, the constraint passes.
    #[serde(rename = "cross")]
    Cross {
        field_a: String,
        field_b: String,
        mapping: std::collections::HashMap<i64, Vec<i64>>,
    },
    /// Conditional field activation: when `field` equals `value`,
    /// force the listed `disable` fields to zero.
    ///
    /// Used for instructions like CUS_NOP where rd/rs1/rs2 are inactive.
    #[serde(rename = "enable_mask")]
    EnableMask {
        /// Trigger field name.
        field: String,
        /// Trigger value — when field equals this value, disable applies.
        value: i64,
        /// Fields to force to zero when trigger matches.
        disable: Vec<String>,
    },
    /// Bitmask constraint: `field & mask` must equal `value`.
    ///
    /// Checks that specific bits of a field are set/cleared.
    /// Used to model bit-field decode conditions (e.g. funct7[1]=1 for Zbt).
    #[serde(rename = "bitmask")]
    Bitmask {
        /// Field name.
        field: String,
        /// Bitmask to apply.
        mask: i64,
        /// Expected value after masking.
        value: i64,
    },
    /// Conditional field assignment: when `field` equals `value`,
    /// assign the listed `set` fields to their specified values.
    ///
    /// Generalization of enable_mask: instead of forcing to zero,
    /// forces to arbitrary per-field values when trigger matches.
    /// Used for instructions where specific fields are tied to fixed values.
    #[serde(rename = "enable_set")]
    EnableSet {
        /// Trigger field name.
        field: String,
        /// Trigger value — when field equals this value, assignment applies.
        value: i64,
        /// Fields and their assigned values when trigger matches.
        set: Vec<FieldAssignment>,
    },
}

/// A single field-to-value assignment for enable_set.
#[derive(Debug, Clone, Deserialize)]
pub struct FieldAssignment {
    /// Field name to assign.
    pub field: String,
    /// Value to assign.
    pub value: i64,
}

/// A projector to resolve from the ProjectorRegistry.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum ProjectorSpec {
    /// Sum all axis values.
    #[serde(rename = "sum")]
    Sum,
    /// Extract a single axis value.
    #[serde(rename = "identity")]
    Identity { field: String },
    /// Classify parity of a single axis.
    #[serde(rename = "parity")]
    Parity { field: String },
    /// Tagma 3-axis decoder projection.
    ///
    /// Packs the Hangul decomposition of the field value into the
    /// golden-anchor layout offset[28:15] i[14:10] m[9:5] f[4:0], with
    /// offset = code - base, i = offset / 588, m = (offset % 588) / 28,
    /// f = offset % 28. Returns None for code points outside the valid
    /// Hangul syllable block, matching the tagma_decoder domain.
    #[serde(rename = "tagma_decode")]
    TagmaDecode {
        /// Field holding the 16-bit Hangul code point.
        field: String,
        /// Base of the Hangul syllable block; defaults to 0xAC00.
        #[serde(default = "default_tagma_base")]
        base: i64,
    },
}

fn default_tagma_base() -> i64 {
    0xAC00
}
