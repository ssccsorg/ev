//! Spike backend — ISA simulation verification via Spike.
//!
//! Implements `RunSimulation` for the Spike RISC-V ISA simulator.
//! Batches all valid encodings into a single RISC-V C program,
//! cross-compiles it, runs the ELF under Spike + pk, and parses
//! per-encoding pass/fail results from stdout.
//!
//! # Architecture
//!
//! 1. Static verification produces pass/fail evaluations.
//! 2. Passing encodings are placed into a C data array with field values.
//! 3. A C harness is generated that:
//!    - Iterates over all valid encodings
//!    - Evaluates each encoding against the spec's constraints (same logic
//!      as ev's Rust evaluator, translated to C)
//!    - Prints per-encoding pass/fail to stdout
//! 4. The C file is cross-compiled with riscv64-unknown-elf-gcc.
//! 5. Spike + pk executes the ELF; stdout contains per-encoding results.
//! 6. Results are merged back into the evaluation list.
//!
//! # Instruction execution (deferred)
//!
//! The current C harness performs only static constraint verification.
//! Actual instruction-word execution under Spike is deferred because
//! Spike's proxy kernel (pk) terminates the process on illegal instruction
//! traps rather than delivering SIGILL to the process. To validate that
//! instruction words decode correctly at the CPU level, a Spike extension
//! plugin (e.g., CVA6 cvxif) is required, or a custom signal-handling
//! approach that works within pk's process model.
//!
//! # Environment variables
//!
//! * `EV_SPIKE_BIN` — path to the Spike binary (default: "spike")
//! * `EV_PK_PATH` — path to the pk proxy kernel (default: "pk")
//! * `EV_RISCV_CC` — RISC-V cross-compiler (default: "riscv64-unknown-elf-gcc")

use crate::evaluate::Evaluation;
use crate::spec::{ConstraintSpec, EncodingLayout, VerificationSpec};
use crate::synth::sim::{RunSimulation, SimulationResult};
use std::collections::BTreeMap;
use std::path::Path;

/// Environment variables for tool discovery.
const EV_SPIKE_BIN: &str = "EV_SPIKE_BIN";
const EV_PK_PATH: &str = "EV_PK_PATH";
const EV_RISCV_CC: &str = "EV_RISCV_CC";

/// Default tool paths.
const DEFAULT_SPIKE: &str = "spike";
const DEFAULT_PK: &str = "pk";
const DEFAULT_CC: &str = "riscv64-unknown-elf-gcc";

/// Spike ISA simulator backend.
pub struct SpikeBackend;

impl RunSimulation for SpikeBackend {
    fn run(
        &self,
        spec: &VerificationSpec,
        static_evaluations: Vec<Evaluation>,
    ) -> anyhow::Result<SimulationResult> {
        let field_names: Vec<&String> = spec.fields.keys().collect();
        let num_fields = field_names.len();

        // Collect valid (passing) encodings while preserving their original indices.
        let valid_indices: Vec<usize> = static_evaluations
            .iter()
            .enumerate()
            .filter(|(_, e)| e.passed)
            .map(|(i, _)| i)
            .collect();
        let valid_rows: Vec<Vec<i64>> = static_evaluations
            .iter()
            .filter(|e| e.passed)
            .map(|e| e.combination.values.clone())
            .collect();

        let field_order: Vec<String> = spec.fields.keys().cloned().collect();

        if valid_rows.is_empty() || num_fields == 0 {
            return Ok(SimulationResult {
                tool: "spike".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                field_order,
                evaluations: static_evaluations,
                extra: None,
            });
        }

        // Generate C source with signal handling and per-encoding execution.
        let tmp_dir = std::env::temp_dir().join("ev-sim");
        std::fs::create_dir_all(&tmp_dir)?;
        let c_src = generate_c_source(
            &spec.target,
            &spec.encoding,
            &field_names,
            &spec.constraints,
            &valid_rows,
        );
        let c_file_name = format!("ev_sim_{}.c", spec.target.replace(char::is_whitespace, "_"));
        let c_path = tmp_dir.join(&c_file_name);
        std::fs::write(&c_path, c_src)?;

        // Cross-compile.
        let elf_name = format!("ev_sim_{}", spec.target.replace(char::is_whitespace, "_"));
        let elf_path = tmp_dir.join(&elf_name);
        cross_compile(&c_path, &elf_path)?;

        // Run under Spike.
        let stdout = run_spike(&elf_path)?;

        // Parse results and merge using original indices.
        let spike_passed = parse_spike_output(&stdout, valid_rows.len());
        let merged = merge_results_with_indices(static_evaluations, &valid_indices, &spike_passed);

        Ok(SimulationResult {
            tool: "spike".into(),
            version: get_spike_version(),
            field_order,
            evaluations: merged,
            extra: None,
        })
    }
}

/// Get Spike version string.
fn get_spike_version() -> String {
    let spike_bin = std::env::var(EV_SPIKE_BIN).unwrap_or_else(|_| DEFAULT_SPIKE.into());
    std::process::Command::new(&spike_bin)
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| {
            String::from_utf8(o.stdout)
                .ok()
                .or_else(|| String::from_utf8(o.stderr).ok())
        })
        .unwrap_or_else(|| "unknown".into())
        .trim()
        .to_string()
}

fn merge_results_with_indices(
    static_evaluations: Vec<Evaluation>,
    valid_indices: &[usize],
    spike_passed: &BTreeMap<usize, bool>,
) -> Vec<Evaluation> {
    let mut spike_map: BTreeMap<usize, bool> = BTreeMap::new();
    for (spike_idx, &orig_idx) in valid_indices.iter().enumerate() {
        if let Some(passed) = spike_passed.get(&spike_idx) {
            spike_map.insert(orig_idx, *passed);
        }
    }

    static_evaluations
        .into_iter()
        .enumerate()
        .map(|(i, mut eval)| {
            if eval.passed {
                match spike_map.get(&i) {
                    Some(true) => {}
                    Some(false) => {
                        eval.passed = false;
                        eval.reason = "Spike: illegal instruction trap".into();
                    }
                    None => {
                        eval.passed = false;
                        eval.reason = "Spike: no result for this encoding".into();
                    }
                }
            }
            eval
        })
        .collect()
}

// ============================================================================
// C source generation
// ============================================================================

/// Generate C source with packed encoding data, constraint evaluation,
/// and instruction word assembly from encoding layout.
fn generate_c_source(
    target: &str,
    encoding_opt: &Option<EncodingLayout>,
    field_names: &[&String],
    constraints: &[ConstraintSpec],
    rows: &[Vec<i64>],
) -> String {
    let num_encodings = rows.len();
    let num_fields = field_names.len();

    // Pack each encoding as a row of comma-separated values.
    let data_lines: Vec<String> = rows
        .iter()
        .map(|row| {
            format!(
                "  {{ {} }}",
                row.iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })
        .collect();

    // Generate field index constants.
    let field_indexes: Vec<String> = field_names
        .iter()
        .enumerate()
        .map(|(i, name)| format!("#define IDX_{} {}", name, i))
        .collect();

    // Generate constraint check code.
    let constraint_code = generate_c_constraints(constraints, field_names);

    // Generate instruction word assembly code from encoding layout.
    // Fields in the layout that are absent from the spec's field list
    // (e.g. a fixed `opcode` field) use constant 0, since they are not
    // part of the combinatorial expansion.
    let assemble_lines: Vec<String> = match encoding_opt {
        Some(layout) => layout
            .field_map
            .iter()
            .map(|(name, mapping)| {
                let mask = (1u64 << mapping.width) - 1;
                let has_idx = field_names.contains(&name);
                if has_idx {
                    format!(
                        "    word |= (uint64_t)(enc[IDX_{name}] & 0x{mask:X}ULL) << {pos};",
                        name = name,
                        mask = mask,
                        pos = mapping.pos
                    )
                } else {
                    format!(
                        "    word |= (uint64_t)(0 & 0x{mask:X}ULL) << {pos};",
                        mask = mask,
                        pos = mapping.pos
                    )
                }
            })
            .collect(),
        None => vec![],
    };
    let assemble_code = if assemble_lines.is_empty() {
        "    (void)word;".into()
    } else {
        assemble_lines.join("\n")
    };

    // Generate pre-computed instruction words for each encoding.
    let instr_word_lines: Vec<String> = match encoding_opt {
        Some(layout) => rows
            .iter()
            .map(|row| {
                let mut word: u64 = 0;
                for (name, mapping) in &layout.field_map {
                    if let Some(idx) = field_names.iter().position(|n| *n == name) {
                        let val = row[idx] as u64;
                        let mask = (1u64 << mapping.width) - 1;
                        word |= (val & mask) << mapping.pos;
                    }
                }
                // Truncate to insn_width bits
                let mask = if layout.insn_width < 64 {
                    (1u64 << layout.insn_width) - 1
                } else {
                    !0u64
                };
                format!("    0x{:016X}ULL", word & mask)
            })
            .collect(),
        None => vec![],
    };
    let instr_word_data = if instr_word_lines.is_empty() {
        String::new()
    } else {
        instr_word_lines.join(",\n")
    };
    let instr_word_array = if !instr_word_lines.is_empty() {
        format!(
            "\n/* Pre-assembled instruction words from encoding layout */\nstatic const uint64_t INSTR_WORDS[{nenc}] = {{\n{data}\n}};\n",
            nenc = num_encodings,
            data = instr_word_data
        )
    } else {
        String::new()
    };

    format!(
        r#"#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>

static void init(void) __attribute__((constructor));
static void init(void) {{ setbuf(stdout, NULL); setbuf(stderr, NULL); }}

/* Auto-generated by ev — ExaVerif Spike backend */
/* Target: {target} */
/* Encodings: {nenc} */
/* Fields: {nfields} */

{field_indexes}

/* Encoding data array — each row holds raw field values */
/* Non-const so enable_mask constraints can force fields to zero at runtime */
static int64_t ENCODINGS[{nenc}][{nfields}] = {{
{data}
}};
{instr_word_array}

const uint64_t NUM_ENCODINGS = {nenc};
const uint64_t NUM_FIELDS = {nfields};

/* Static constraint check (same as ev's evaluate_all) */
/* Non-const to allow enable_mask to force fields to zero */
static int check_encoding(int64_t enc[]) {{
{constraint_code}
}}

static uint64_t assemble_instr(const int64_t enc[]) {{
    uint64_t word = 0;
{assemble_code}
    return word;
}}

int main(void) {{
    uint64_t pass = 0, fail = 0;
    for (uint64_t i = 0; i < NUM_ENCODINGS; i++) {{
        int ok = check_encoding(ENCODINGS[i]);
        uint64_t instr = assemble_instr(ENCODINGS[i]);
        if (ok) {{ pass++; }} else {{ fail++; }}
        printf("ENC:%llu:%d:0x%016llX\n", (unsigned long long)i, ok, (unsigned long long)instr);
    }}
    printf("PASSED:%llu\n", (unsigned long long)pass);
    printf("FAILED:%llu\n", (unsigned long long)fail);
    return fail > 0 ? 1 : 0;
}}
"#,
        target = target,
        nenc = num_encodings,
        nfields = num_fields,
        data = data_lines.join(",\n"),
        field_indexes = field_indexes.join("\n"),
        constraint_code = constraint_code,
        instr_word_array = instr_word_array,
        assemble_code = assemble_code,
    )
}

/// Generate C code for constraint checking from a list of ConstraintSpec.
fn generate_c_constraints(constraints: &[ConstraintSpec], field_names: &[&String]) -> String {
    if constraints.is_empty() {
        return "    (void)enc;\n    return 1;".into();
    }

    let mut lines: Vec<String> = Vec::new();
    for constraint in constraints {
        let cond = generate_c_constraint_expr(constraint, field_names);
        if !cond.is_empty() {
            lines.push(cond);
        }
    }

    if lines.is_empty() {
        "    (void)enc;\n    return 1;".into()
    } else {
        lines.push("    return 1;".into());
        lines.join("\n")
    }
}

fn generate_c_constraint_expr(constraint: &ConstraintSpec, _field_names: &[&String]) -> String {
    match constraint {
        ConstraintSpec::Range { field, min, max } => {
            format!(
                "    if (enc[IDX_{0}] < {min} || enc[IDX_{0}] > {max}) return 0;",
                field,
                min = min,
                max = max
            )
        }
        ConstraintSpec::Even { field } => {
            format!("    if (enc[IDX_{}] & 1) return 0;", field)
        }
        ConstraintSpec::Eq { field_a, field_b } => {
            format!(
                "    if (enc[IDX_{0}] != enc[IDX_{1}]) return 0;",
                field_a, field_b
            )
        }
        ConstraintSpec::Neq { field_a, field_b } => {
            format!(
                "    if (enc[IDX_{0}] == enc[IDX_{1}]) return 0;",
                field_a, field_b
            )
        }
        ConstraintSpec::Lt { field, value } => {
            format!("    if (enc[IDX_{}] >= {}) return 0;", field, value)
        }
        ConstraintSpec::Gt { field, value } => {
            format!("    if (enc[IDX_{}] <= {}) return 0;", field, value)
        }
        ConstraintSpec::Le { field, value } => {
            format!("    if (enc[IDX_{}] > {}) return 0;", field, value)
        }
        ConstraintSpec::Ge { field, value } => {
            format!("    if (enc[IDX_{}] < {}) return 0;", field, value)
        }
        ConstraintSpec::Oneof { field, values } => {
            let or_exprs: Vec<String> = values
                .iter()
                .map(|v| format!("enc[IDX_{0}] == {v}", field, v = v))
                .collect();
            format!("    if (!({})) return 0;", or_exprs.join(" || "))
        }
        ConstraintSpec::Cross {
            field_a,
            field_b,
            mapping,
        } => {
            let mut cases: Vec<String> = mapping
                .iter()
                .map(|(va, vbs)| {
                    let set = vbs
                        .iter()
                        .map(|vb| format!("enc[IDX_{0}] == {vb}", field_b, vb = vb))
                        .collect::<Vec<_>>()
                        .join(" || ");
                    format!(
                        "        case {va}:\n            if (!({set})) return 0;\n            break;",
                        va = va, set = set
                    )
                })
                .collect();
            cases.push("        default:\n            break;".to_string());
            format!(
                "    switch (enc[IDX_{0}]) {{\n{cases}\n    }}",
                field_a,
                cases = cases.join("\n")
            )
        }
        ConstraintSpec::EnableMask {
            field,
            value,
            disable,
        } => {
            // enable_mask: when trigger field matches, force disabled fields to 0.
            let forced_zeros: Vec<String> = disable
                .iter()
                .map(|f| format!("    enc[IDX_{f}] = 0;", f = f))
                .collect();
            if forced_zeros.is_empty() {
                String::new()
            } else {
                format!(
                    "    if (enc[IDX_{field}] == {value}) {{\n{body}\n    }}",
                    field = field,
                    value = value,
                    body = forced_zeros.join("\n")
                )
            }
        }
    }
}

// ============================================================================
// Cross-compilation and Spike execution
// ============================================================================

fn cross_compile(c_path: &Path, elf_path: &Path) -> anyhow::Result<()> {
    let cc = std::env::var(EV_RISCV_CC).unwrap_or_else(|_| DEFAULT_CC.into());
    let status = std::process::Command::new(&cc)
        .args(["-static", "-Wall", "-Wextra", "-O0", "-g", "-o"])
        .arg(elf_path)
        .arg(c_path)
        .status()
        .map_err(|e| anyhow::anyhow!("failed to run cross-compiler '{}': {}", cc, e))?;
    if !status.success() {
        anyhow::bail!("cross-compilation failed (exit: {})", status);
    }
    Ok(())
}

fn run_spike(elf_path: &Path) -> anyhow::Result<String> {
    let spike_bin = std::env::var(EV_SPIKE_BIN).unwrap_or_else(|_| DEFAULT_SPIKE.into());
    let pk_path = std::env::var(EV_PK_PATH).unwrap_or_else(|_| DEFAULT_PK.into());
    let output = std::process::Command::new(&spike_bin)
        .args([&pk_path])
        .arg(elf_path)
        .output()
        .map_err(|e| anyhow::anyhow!("failed to run spike '{}': {}", spike_bin, e))?;
    if !output.status.success() && output.status.code() != Some(1) {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "spike execution failed (exit: {}): {}",
            output.status,
            stderr
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn parse_spike_output(stdout: &str, num_encodings: usize) -> BTreeMap<usize, bool> {
    let mut results = BTreeMap::new();
    for line in stdout.lines() {
        if let Some(rest) = line.strip_prefix("ENC:") {
            let parts: Vec<&str> = rest.split(':').collect();
            if parts.len() >= 2 {
                if let (Ok(idx), Ok(ok)) = (parts[0].parse::<usize>(), parts[1].parse::<u8>()) {
                    if idx < num_encodings {
                        results.insert(idx, ok != 0);
                    }
                }
            }
        }
    }
    results
}
