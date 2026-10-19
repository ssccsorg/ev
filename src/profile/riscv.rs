//! RISC-V instruction layouts, compiled under the `riscv` feature.
//!
//! These are profiles the engine treats as data. A build without the feature
//! has no RISC-V layout at all, and a spec that names one fails to parse.

use super::LayoutRegistry;
use crate::spec::{FieldBitMapping, WordLayout};
use std::collections::BTreeMap;

/// Register the RISC-V profiles. `r4` is the name older fixtures used.
pub fn register(registry: &mut LayoutRegistry) {
    registry.register("riscv-r", r_type);
    registry.register("riscv-r4", r4);
    registry.register("r4", r4);
}

/// R-type: opcode[6:0] rd[11:7] funct3[14:12] rs1[19:15] rs2[24:20] funct7[31:25].
fn r_type() -> WordLayout {
    layout(&[
        ("opcode", 0, 7),
        ("rd", 7, 5),
        ("funct3", 12, 3),
        ("rs1", 15, 5),
        ("rs2", 20, 5),
        ("funct7", 25, 7),
    ])
}

/// R4: R-type with func2[26:25] replacing funct7 and rs3[31:27].
fn r4() -> WordLayout {
    layout(&[
        ("opcode", 0, 7),
        ("rd", 7, 5),
        ("funct3", 12, 3),
        ("rs1", 15, 5),
        ("rs2", 20, 5),
        ("func2", 25, 2),
        ("rs3", 27, 5),
    ])
}

fn layout(fields: &[(&str, u32, u32)]) -> WordLayout {
    WordLayout {
        word_width: Some(32),
        field_map: fields
            .iter()
            .map(|(name, pos, width)| {
                (
                    (*name).to_string(),
                    FieldBitMapping {
                        pos: *pos,
                        width: *width,
                    },
                )
            })
            .collect::<BTreeMap<_, _>>(),
    }
}
