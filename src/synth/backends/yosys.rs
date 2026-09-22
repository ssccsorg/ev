//! YosysBackend — runs Yosys synthesis via CLI.
//!
//! This is the only file in the codebase that knows about Yosys.
//! All external-tool coupling is isolated here behind the `RunSynthesis` trait.
//!
//! # Dependencies
//!
//! * `yosys` must be installed and available in `PATH`.

use crate::synth::{error_report, RunSynthesis, SynthesisMetrics};
use std::path::Path;
use std::process::Command;

/// Yosys synthesis backend.
///
/// Invokes `yosys` as a subprocess with the following Tcl script:
///
/// ```tcl
/// read_verilog -sv <file>
/// hierarchy -top <module>
/// proc
/// synth -top <module>
/// stat -json > <log>
/// write_json <netlist>
/// show -format dot -prefix <dir>/netlist <module>
/// ```
///
/// Parses the JSON statistics output and produces `SynthesisMetrics`.
pub struct YosysBackend;

impl RunSynthesis for YosysBackend {
    fn run(&self, rtl_path: &Path, top_module: &str) -> anyhow::Result<SynthesisMetrics> {
        let source = rtl_path.to_string_lossy().to_string();
        let module_name = top_module.to_string();

        // Verify Yosys availability first.
        let yosys_bin = find_yosys()?;

        // Capture version string.
        let version = Command::new(&yosys_bin)
            .arg("--version")
            .output()
            .ok()
            .and_then(|o| {
                String::from_utf8(o.stdout)
                    .ok()
                    .map(|s| s.split('\n').next().unwrap_or("unknown").to_string())
            })
            .unwrap_or_else(|| "unknown".to_string());

        // Create a temporary working directory.
        let work_dir = tempfile::TempDir::new()?;
        let work_path = work_dir.path();

        let stat_path = work_path.join("stat.json");
        let netlist_path = work_path.join("netlist.json");
        let dot_path = work_path.join("netlist.dot");
        let log_path = work_path.join("yosys.log");

        let sv_path_str = rtl_path.display().to_string();
        // NOTE: Yosys -p expects a string that may contain whitespace.
        // Quoting module names with double quotes works in Yosys Tcl scripts but
        // not in -p inline commands — the Yosys -p parser does not interpret Tcl
        // string quoting the same way. So keep module names unquoted.
        let script = format!(
            "read_verilog -sv {sv}; hierarchy -top {top}; proc; synth -top {top}; \
             tee -o {stat} stat -json; write_json {netlist}; \
             show -format dot -prefix {prefix} {top};",
            sv = sv_path_str,
            top = top_module,
            stat = stat_path.display(),
            netlist = netlist_path.display(),
            prefix = work_path.join("netlist").display(),
        );

        let result = Command::new(&yosys_bin)
            .args(["-l", &log_path.to_string_lossy()])
            .args(["-p", &script])
            .output()
            .map_err(|e| anyhow::anyhow!("Failed to execute yosys: {}", e))?;

        if !result.status.success() {
            let mut detail = String::new();
            detail.push_str(&String::from_utf8_lossy(&result.stderr));
            detail.push_str(&String::from_utf8_lossy(&result.stdout));
            // Also read the Yosys log if available.
            if log_path.exists() {
                if let Ok(log) = std::fs::read_to_string(&log_path) {
                    detail.push_str("\n--- yosys log ---\n");
                    detail.push_str(&log);
                }
            }
            return Ok(error_report(
                "yosys",
                &version,
                rtl_path,
                top_module,
                format!(
                    "Yosys exited with code {:?}: {}",
                    result.status.code(),
                    detail
                ),
            ));
        }

        // ── Parse statistics ──────────────────────────────────────────
        // gate_count and cell_area live in the core struct; everything
        // else (cell types, DOT path, warnings) goes into `extra` as
        // opaque backend-specific data.
        let stat_data: Option<serde_json::Value> = if stat_path.exists() {
            std::fs::read_to_string(&stat_path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
        } else {
            None
        };

        let summary = parse_stat(stat_data.as_ref(), top_module);
        let gate_count = summary.gate_count;
        let cell_area = summary.cell_area;
        let mut extra = serde_json::json!({});

        if let Some(cell_types) = summary.cell_types {
            extra["cell_types"] = serde_json::Value::Object(cell_types);
        }

        // ── DOT output ────────────────────────────────────────────────
        if dot_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&dot_path) {
                if content.contains("digraph") {
                    extra["dot_path"] = serde_json::json!(dot_path.to_string_lossy().to_string());
                }
            }
        }

        extra["netlist_path"] = serde_json::json!(netlist_path.to_string_lossy().to_string());

        // ── Warnings ──────────────────────────────────────────────────
        if log_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&log_path) {
                let warns: Vec<String> = content
                    .lines()
                    .filter(|l| l.to_lowercase().contains("warning"))
                    .map(|l| l.trim().to_string())
                    .collect();
                if !warns.is_empty() {
                    extra["warnings"] = serde_json::Value::Array(
                        warns.into_iter().map(serde_json::Value::String).collect(),
                    );
                }
            }
        }

        Ok(SynthesisMetrics {
            tool: "yosys".into(),
            version,
            source,
            module_name,
            gate_count,
            cell_area,

            extra: Some(extra),
            status: "ok".into(),
            message: None,
        })
    }
}

/// Find `yosys` in PATH, returning a user-friendly error if not found.
fn find_yosys() -> anyhow::Result<std::path::PathBuf> {
    // `which::which` is not available as a dependency, so we search PATH manually.
    std::env::var_os("PATH")
        .ok_or_else(|| anyhow::anyhow!("PATH is not set"))?
        .to_string_lossy()
        .split(':')
        .filter_map(|dir| {
            let candidate = Path::new(dir).join("yosys");
            if candidate.is_file() {
                // On Unix, also check executable bit.
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    candidate
                        .metadata()
                        .ok()
                        .filter(|m| m.permissions().mode() & 0o111 != 0)
                        .map(|_| candidate.clone())
                }
                #[cfg(not(unix))]
                {
                    Some(candidate)
                }
            } else {
                None
            }
        })
        .next()
        .ok_or_else(|| {
            anyhow::anyhow!("yosys not found in PATH. Install yosys or set PATH accordingly.")
        })
}

/// Metrics read out of a yosys `stat -json` report.
#[derive(Debug, Default, PartialEq)]
struct StatSummary {
    gate_count: Option<u64>,
    cell_area: Option<f64>,
    cell_types: Option<serde_json::Map<String, serde_json::Value>>,
}

/// Read the gate count, cell area, and cell types out of a `stat -json`
/// report.
///
/// Yosys writes aggregate counts under `design` and per-module counts under
/// `modules`, keyed by the module name in plain or escaped form (`top` or
/// `\top`). The aggregate is preferred for the gate count, because it is the
/// number the syntagma synthesis scripts report for the same RTL. The module
/// entry is the fallback, and it is the source of the cell types, so
/// `gate_count` covers the design including cells inside submodules while
/// `cell_types` lists the top module's own cells.
fn parse_stat(stat: Option<&serde_json::Value>, top_module: &str) -> StatSummary {
    let Some(stat) = stat else {
        return StatSummary::default();
    };

    let aggregate = stat.get("design");
    let module = stat.get("modules").and_then(|modules| {
        modules
            .get(top_module)
            .or_else(|| modules.get(format!("\\{top_module}")))
    });

    let value = |key: &str| {
        aggregate
            .and_then(|stats| stats.get(key))
            .or_else(|| module.and_then(|stats| stats.get(key)))
    };

    StatSummary {
        gate_count: value("num_cells").and_then(|v| v.as_u64()),
        cell_area: value("area").and_then(|v| v.as_f64()),
        cell_types: module
            .and_then(|stats| stats.get("num_cells_by_type"))
            .and_then(|types| types.as_object())
            .map(|types| {
                types
                    .iter()
                    .filter(|(_, count)| count.as_u64().is_some_and(|n| n > 0))
                    .map(|(name, count)| (name.clone(), count.clone()))
                    .collect::<serde_json::Map<_, _>>()
            })
            .filter(|types| !types.is_empty()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Shaped like `yosys -p "...; stat -json"` on yosys 0.65: aggregate
    /// counts under `design`, per-module counts under the escaped name.
    fn yosys_stat_report() -> serde_json::Value {
        serde_json::json!({
            "creator": "Yosys 0.65",
            "design": {
                "num_cells": 478,
                "num_wires": 480,
                "num_cells_by_type": { "$_AND_": 82, "$_XOR_": 71 },
            },
            "modules": {
                "\\tagma_decoder": {
                    "num_cells": 478,
                    "num_cells_by_type": { "$_AND_": 82, "$_XOR_": 71 },
                },
            },
        })
    }

    #[test]
    fn parse_stat_reads_the_aggregate_and_the_cell_types() {
        let report = yosys_stat_report();
        let summary = parse_stat(Some(&report), "tagma_decoder");

        assert_eq!(summary.gate_count, Some(478));
        assert_eq!(summary.cell_area, None, "area needs -liberty");
        let types = summary.cell_types.expect("cell types are reported");
        assert_eq!(types.len(), 2);
        assert_eq!(types.get("$_AND_"), Some(&serde_json::json!(82)));
    }

    #[test]
    fn parse_stat_falls_back_to_the_module_entry() {
        let mut report = yosys_stat_report();
        report.as_object_mut().expect("object").remove("design");

        let summary = parse_stat(Some(&report), "tagma_decoder");
        assert_eq!(summary.gate_count, Some(478));
    }

    #[test]
    fn parse_stat_handles_a_missing_report() {
        let summary = parse_stat(None, "tagma_decoder");
        assert_eq!(summary, StatSummary::default());
    }

    #[test]
    fn parse_stat_ignores_a_module_that_is_not_in_the_report() {
        let report = yosys_stat_report();
        let mut report = report.clone();
        report.as_object_mut().expect("object").remove("design");

        let summary = parse_stat(Some(&report), "other_module");
        assert_eq!(summary.gate_count, None);
        assert_eq!(summary.cell_types, None);
    }

    /// The parse against a report Yosys actually wrote, so the pinned shape
    /// is not only the hand-written one above.
    ///
    /// Captured verbatim with Yosys 0.65 from
    /// `tests/fixtures/rtl/decode_demo.v`: read_verilog, hierarchy, proc,
    /// synth, `tee -o <file> stat -json`.
    #[test]
    fn parse_stat_reads_a_captured_report() {
        let raw = std::fs::read_to_string("tests/fixtures/yosys/stat_decode_demo.json")
            .expect("captured stat report fixture must be readable");
        let report: serde_json::Value =
            serde_json::from_str(&raw).expect("the captured report must be JSON");

        let summary = parse_stat(Some(&report), "decode_demo");

        assert_eq!(summary.gate_count, Some(16));
        assert_eq!(summary.cell_area, None, "area needs -liberty");
        let types = summary.cell_types.expect("cell types are reported");
        assert_eq!(types.get("$_AND_"), Some(&serde_json::json!(5)));
        assert_eq!(types.get("$_NAND_"), Some(&serde_json::json!(1)));
        assert_eq!(types.get("$_XNOR_"), Some(&serde_json::json!(7)));
        assert_eq!(types.get("$_XOR_"), Some(&serde_json::json!(3)));
    }
}
