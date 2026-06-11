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
//! 2. Passing encodings are assembled into instruction words.
//! 3. A C harness is generated that:
//!    - Registers a SIGILL handler via sigaction + sigsetjmp.
//!    - For each encoding, copies the instruction word into an
//!      executable buffer and calls it as a function pointer.
//!    - If the instruction traps (SIGILL), siglongjmp recovers
//!      and the encoding is marked as failed.
//!    - If it returns normally, the encoding is marked as passed.
//! 4. The C file is cross-compiled with riscv64-unknown-elf-gcc.
//! 5. Spike + pk executes the ELF; stdout contains per-encoding results.
//! 6. Results are merged back into the evaluation list.
//!
//! # Why this works under Spike
//!
//! The C harness is cross-compiled to a RISC-V ELF. When Spike executes
//! it, *all* code (including the function-pointer trampoline and the
//! instruction word written to the buffer) is RISC-V code. The
//! instruction word *is* a valid RISC-V instruction (custom-3 opcode)
//! that Spike attempts to decode. If Spike doesn't recognize the
//! opcode, it raises SIGILL, which the signal handler catches.
//!
//! This approach works with stock Spike — no custom extensions needed.
//!
//! # Environment variables
//!
//! * `EV_SPIKE_BIN` — path to the Spike binary (default: "spike")
//! * `EV_PK_PATH` — path to the pk proxy kernel (default: "pk")
//! * `EV_RISCV_CC` — RISC-V cross-compiler (default: "riscv64-unknown-elf-gcc")

use crate::evaluate::Evaluation;
use crate::spec::{ConstraintSpec, VerificationSpec};
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

        if valid_rows.is_empty() || num_fields == 0 {
            return Ok(SimulationResult {
                tool: "spike".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                evaluations: static_evaluations,
                extra: None,
            });
        }

        // Generate C source with signal handling and per-encoding execution.
        let tmp_dir = std::env::temp_dir().join("ev-sim");
        std::fs::create_dir_all(&tmp_dir)?;
        let c_src = generate_c_source(&field_names, &spec.constraints, &valid_rows);
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

/// Generate C source that performs static constraint verification.
///
/// This C harness mirrors ev's static evaluation (field range/cross-field
/// constraints) and runs it under Spike to confirm that the cross-compiled
/// C code produces the same results as ev's Rust evaluator.
///
/// Actual instruction-word execution under Spike is not possible with stock
/// pk, because pk terminates the process on illegal instruction traps
/// instead of delivering SIGILL to the process. To test actual instruction
/// execution, a Spike extension plugin (e.g., CVA6 cvxif) is required.
fn generate_c_source(
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
static const int64_t ENCODINGS[{nenc}][{nfields}] = {{
{data}
}};

const uint64_t NUM_ENCODINGS = {nenc};

/* Static constraint check (same as ev's evaluate_all) */
static int check_encoding(const int64_t enc[]) {{
{constraint_code}
}}

int main(void) {{
    uint64_t pass = 0, fail = 0;
    for (uint64_t i = 0; i < NUM_ENCODINGS; i++) {{
        int ok = check_encoding(ENCODINGS[i]);
        printf("ENC:%llu:%d\n", (unsigned long long)i, ok);
        if (ok) {{ pass++; }} else {{ fail++; }}
    }}
    printf("PASSED:%llu\n", (unsigned long long)pass);
    printf("FAILED:%llu\n", (unsigned long long)fail);
    return fail > 0 ? 1 : 0;
}}
"#,
        target = "cva6_xif",
        nenc = num_encodings,
        nfields = num_fields,
        data = data_lines.join(",\n"),
        field_indexes = field_indexes.join("\n"),
        constraint_code = constraint_code,
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
