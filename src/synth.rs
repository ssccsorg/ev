//! Synthesis capability — runs external synthesis tools and produces reports.
//!
//! Each backend implements the `SynthesisCapable` trait. The pipeline depends
//! on the trait, not on any specific tool.

use crate::spec::VerificationSpec;
use serde::{Deserialize, Serialize};

/// Report produced by a synthesis backend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthesisReport {
    pub tool: String,
    pub version: String,
    pub source: String,
    pub module_name: String,
    pub gate_count: Option<u64>,
    pub cell_area: Option<f64>,
    pub status: String,
    pub message: Option<String>,
}

/// Capability: synthesize a design from a verification spec.
pub trait SynthesisCapable: Send + Sync {
    fn synthesize(&self, spec: &VerificationSpec) -> anyhow::Result<SynthesisReport>;
}

/// Backend that runs an external shell script.
pub struct ScriptBackend {
    script_path: std::path::PathBuf,
}

impl ScriptBackend {
    pub fn new(script_path: std::path::PathBuf) -> Self {
        Self { script_path }
    }
}

impl SynthesisCapable for ScriptBackend {
    fn synthesize(&self, spec: &VerificationSpec) -> anyhow::Result<SynthesisReport> {
        let sv_path = generate_sv(spec)?;
        let output = std::process::Command::new("bash")
            .arg(&self.script_path)
            .arg(&sv_path)
            .arg(&spec.target)
            .output()
            .map_err(|e| anyhow::anyhow!("Failed to run synthesis: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Ok(SynthesisReport {
                tool: "unknown".into(),
                version: "unknown".into(),
                source: sv_path.to_string_lossy().to_string(),
                module_name: spec.target.clone(),
                gate_count: None,
                cell_area: None,
                status: "error".into(),
                message: Some(stderr.to_string()),
            });
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let report: SynthesisReport = serde_json::from_str(&stdout)
            .map_err(|e| anyhow::anyhow!("Failed to parse synthesis JSON: {}", e))?;
        Ok(report)
    }
}

fn generate_sv(spec: &VerificationSpec) -> anyhow::Result<std::path::PathBuf> {
    let tmp_dir = std::env::temp_dir().join("ev-synth");
    std::fs::create_dir_all(&tmp_dir)?;
    let sv_path = tmp_dir.join(format!("{}.sv", spec.target));
    let mut sv = String::new();
    sv.push_str("// Auto-generated\n");
    sv.push_str(&format!("module {} (\n", spec.target));
    sv.push_str("  input logic [63:0] coord,\n  output logic result\n);\n");
    sv.push_str("  assign result = 1'b1;\nendmodule\n");
    std::fs::write(&sv_path, sv)?;
    Ok(sv_path)
}

/// Run the default synthesis backend.
pub fn synthesize_default(spec: &VerificationSpec) -> anyhow::Result<SynthesisReport> {
    let script_path = if let Ok(path) = std::env::var("EV_SYNTH_SCRIPT") {
        std::path::PathBuf::from(path)
    } else {
        let candidates = [
            std::env::current_exe().ok().and_then(|p| {
                p.parent().map(|p| p.join("../scripts/synth/default-synth.sh"))
            }),
            Some(std::path::PathBuf::from("scripts/synth/default-synth.sh")),
        ];
        let mut found = None;
        for c in candidates.iter().flatten() {
            if c.exists() {
                found = Some(c.clone());
                break;
            }
        }
        found.ok_or_else(|| {
            anyhow::anyhow!(
                "No synthesis script. Set EV_SYNTH_SCRIPT or install \
                 scripts/synth/default-synth.sh"
            )
        })?
    };
    ScriptBackend::new(script_path).synthesize(spec)
}
