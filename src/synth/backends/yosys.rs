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
             stat -json > {stat}; write_json {netlist}; \
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

        let mut gate_count: Option<u64> = None;
        let mut cell_area: Option<f64> = None;
        let mut extra = serde_json::json!({});

        if let Some(ref data) = stat_data {
            let top = data.get("top_module").or_else(|| data.get("design"));
            gate_count = top
                .and_then(|t| t.get("num_cells"))
                .and_then(|v| v.as_u64());
            cell_area = top.and_then(|t| t.get("area")).and_then(|v| v.as_f64());
            let cell_types = data
                .get("modules")
                .and_then(|m| m.get(top_module))
                .and_then(|m| m.get("cells"))
                .and_then(|c| c.as_object())
                .map(|obj| {
                    obj.iter()
                        .filter(|(_, v)| v.as_u64().is_some_and(|n| n > 0))
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect::<serde_json::Map<_, _>>()
                })
                .filter(|m| !m.is_empty())
                .map(serde_json::Value::Object);

            if let Some(ct) = cell_types {
                extra["cell_types"] = ct;
            }
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
