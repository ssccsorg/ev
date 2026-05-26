//! Synthesis capability — runs external synthesis tools and produces metrics.
//!
//! Fine-grained capability traits following the neXus model:
//!   GenerateRtl  — spec → RTL file
//!   RunSynthesis — RTL file → tool output
//!   FullSynthesis = GenerateRtl + RunSynthesis (blanket impl)
//!
//! Each backend implements only what it provides. The pipeline depends on
//! traits, not on any specific tool.

use crate::spec::VerificationSpec;
use serde::{Deserialize, Serialize};

/// Tool-agnostic synthesis metrics.
///
/// Every synthesis backend produces these fields. The `netlist_path` field
/// is the handoff point to downstream colonies (OpenROAD, place-and-route).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthesisMetrics {
    pub tool: String,
    pub version: String,
    pub source: String,
    pub module_name: String,
    pub gate_count: Option<u64>,
    pub cell_area: Option<f64>,
    /// Synthesized netlist — handoff point to physical design.
    pub netlist_path: Option<String>,
    /// Gate-level DOT diagram path (Yosys show -format dot).
    pub dot_path: Option<String>,
    /// Per-cell-type breakdown: {"$_AND_": 12, "$_DFF_": 4, ...}.
    #[serde(default)]
    pub cell_types: Option<serde_json::Value>,
    /// Yosys warnings captured from log.
    #[serde(default)]
    pub warnings: Option<serde_json::Value>,
    pub status: String,
    pub message: Option<String>,
}

impl From<SynthesisMetrics> for crate::fih::Fact {
    fn from(m: SynthesisMetrics) -> Self {
        let payload = serde_json::json!({
            "tool": m.tool,
            "version": m.version,
            "source": m.source,
            "gate_count": m.gate_count,
            "cell_area": m.cell_area,
            "netlist_path": m.netlist_path,
            "dot_path": m.dot_path,
            "cell_types": m.cell_types,
            "warnings": m.warnings,
            "status": m.status,
            "message": m.message,
        });
        crate::fih::Fact::new(
            "synthesis_result",
            "ev/synthesis",
            &m.module_name,
            payload,
        )
    }
}

/// Capability: generate RTL source from a verification spec.
///
/// Decoupled from synthesis so any RTL dialect (SystemVerilog, Verilog,
/// VHDL, Chisel) can feed any synthesis backend.
pub trait GenerateRtl: Send + Sync {
    fn generate(&self, spec: &VerificationSpec) -> anyhow::Result<std::path::PathBuf>;
}

/// Capability: run a synthesis tool on an RTL file and produce metrics.
///
/// The tool is invoked with an RTL path and a top-level module name.
/// Backends implement only this trait — they don't know about VerificationSpec.
pub trait RunSynthesis: Send + Sync {
    fn run(&self, rtl_path: &std::path::Path, top_module: &str) -> anyhow::Result<SynthesisMetrics>;
}

/// Aggregate: full synthesis pipeline from spec to metrics.
///
/// Backends that provide both RTL generation and tool execution get this
/// blanket implementation automatically.
#[allow(dead_code)]
pub trait FullSynthesis: GenerateRtl + RunSynthesis {}
impl<T: GenerateRtl + RunSynthesis> FullSynthesis for T {}

/// Compose GenerateRtl + RunSynthesis into a single pipeline call.
#[allow(dead_code)]
pub fn synthesize_pipeline(
    pipeline: &dyn FullSynthesis,
    spec: &VerificationSpec,
) -> anyhow::Result<SynthesisMetrics> {
    let rtl_path = pipeline.generate(spec)?;
    pipeline.run(&rtl_path, &spec.target)
}

/// Default RTL generator — produces SystemVerilog.
pub struct SvGenerator;

impl GenerateRtl for SvGenerator {
    fn generate(&self, spec: &VerificationSpec) -> anyhow::Result<std::path::PathBuf> {
        generate_sv(spec)
    }
}

/// Backend that runs an external shell script.
///
/// Implements `RunSynthesis` only. Pair with a `GenerateRtl` impl
/// (default: `SvGenerator`) to get `FullSynthesis` via blanket impl.
pub struct ScriptBackend {
    script_path: std::path::PathBuf,
}

impl ScriptBackend {
    pub fn new(script_path: std::path::PathBuf) -> Self {
        Self { script_path }
    }
}

impl RunSynthesis for ScriptBackend {
    fn run(&self, rtl_path: &std::path::Path, top_module: &str) -> anyhow::Result<SynthesisMetrics> {
        let output = std::process::Command::new("bash")
            .arg(&self.script_path)
            .arg(rtl_path)
            .arg(top_module)
            .output()
            .map_err(|e| anyhow::anyhow!("Failed to run synthesis: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Ok(SynthesisMetrics {
                tool: "unknown".into(),
                version: "unknown".into(),
                source: rtl_path.to_string_lossy().to_string(),
                module_name: top_module.into(),
                gate_count: None,
                cell_area: None,
                netlist_path: None,
                dot_path: None,
                cell_types: None,
                warnings: None,
                status: "error".into(),
                message: Some(stderr.to_string()),
            });
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let metrics: SynthesisMetrics = serde_json::from_str(&stdout)
            .map_err(|e| anyhow::anyhow!("Failed to parse synthesis JSON: {}", e))?;
        Ok(metrics)
    }
}

/// Mock backend for CI/testing — validates the pipeline without a real tool.
///
/// Reads the generated RTL file, verifies it contains a valid module
/// declaration, and returns deterministic mock metrics. Used when
/// `EV_SYNTH_BACKEND=mock` or directly in unit tests.
pub struct MockSynthesisBackend;

impl RunSynthesis for MockSynthesisBackend {
    fn run(&self, rtl_path: &std::path::Path, top_module: &str) -> anyhow::Result<SynthesisMetrics> {
        let content = std::fs::read_to_string(rtl_path)
            .map_err(|e| anyhow::anyhow!("Mock: cannot read RTL file {}: {}", rtl_path.display(), e))?;

        if !content.contains(&format!("module {}", top_module)) {
            anyhow::bail!(
                "Mock: RTL file {} does not contain module {}",
                rtl_path.display(),
                top_module
            );
        }

        Ok(SynthesisMetrics {
            tool: "mock".into(),
            version: "0.0.0".into(),
            source: rtl_path.to_string_lossy().to_string(),
            module_name: top_module.into(),
            gate_count: Some(0),
            cell_area: None,
            netlist_path: None,
            dot_path: None,
            cell_types: None,
            warnings: None,
            status: "ok".into(),
            message: None,
        })
    }
}

fn generate_sv(spec: &VerificationSpec) -> anyhow::Result<std::path::PathBuf> {
    let tmp_dir = std::env::temp_dir().join("ev-synth");
    std::fs::create_dir_all(&tmp_dir)?;
    // Use a unique suffix so concurrent or sequential runs don't collide.
    let unique_suffix: String = std::iter::repeat_with(fast_random_char)
        .take(8)
        .collect();
    let sv_path = tmp_dir.join(format!("{}_{}.sv", spec.target, unique_suffix));

    if spec.fields.is_empty() {
        anyhow::bail!("Cannot generate SystemVerilog: spec has no fields");
    }

    let mut sv = String::new();
    sv.push_str("// Auto-generated by ev — ExaVerif\n");
    sv.push_str(&format!("module {} (\n", spec.target));

    // Port declarations — one input per field, plus a result output.
    let field_names: Vec<&String> = spec.fields.keys().collect();
    let mut ports: Vec<String> = Vec::new();
    for name in &field_names {
        let field = &spec.fields[*name];
        let width = field_bit_width(field);
        ports.push(format!("  input logic [{}:0] {}", width.saturating_sub(1), name));
    }
    // Result width: generous ceiling based on field count and max width.
    let max_field_width = spec.fields.values().map(field_bit_width).max().unwrap_or(1);
    let result_width = max_field_width + (spec.fields.len() as u32).next_power_of_two().ilog2();
    ports.push(format!("  output logic [{}:0] result", result_width.saturating_sub(1)));
    sv.push_str(&ports.join(",\n"));
    sv.push_str("\n);\n\n");

    // Projector logic.
    let projector_expr = sv_projector(&spec.projector, &field_names);
    sv.push_str(&format!("  assign result = {};\n", projector_expr));

    // Constraint assertions.
    for constraint in &spec.constraints {
        sv.push_str(&format!("\n  {}", sv_constraint_assertion(constraint, &field_names)));
    }

    sv.push_str("\nendmodule\n");
    std::fs::write(&sv_path, sv)?;
    Ok(sv_path)
}

/// Minimum bit width needed to represent a field's domain.
fn field_bit_width(field: &crate::spec::FieldSpec) -> u32 {
    if let Some(ref values) = field.values {
        if values.is_empty() {
            return 1;
        }
        let max_val = values.iter().max().unwrap_or(&0).unsigned_abs();
        let min_val = values.iter().min().unwrap_or(&0).unsigned_abs();
        let span = max_val.max(min_val);
        return bits_for(span);
    }
    if let Some((min, max)) = field.range {
        let span = (max - min).unsigned_abs().max(max.unsigned_abs()).max(min.unsigned_abs());
        return bits_for(span);
    }
    8 // default
}

fn bits_for(value: u64) -> u32 {
    if value == 0 {
        return 1;
    }
    64 - value.leading_zeros()
}

/// Fast non-crypto random char for temp file suffixes.
fn fast_random_char() -> char {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    let h = RandomState::new().build_hasher().finish();
    (b'a' + (h % 26) as u8) as char
}

/// Generate the right-hand side expression for the projector.
fn sv_projector(proj: &crate::spec::ProjectorSpec, field_names: &[&String]) -> String {
    match proj {
        crate::spec::ProjectorSpec::Sum => {
            field_names.iter().map(|n| n.as_str()).collect::<Vec<_>>().join(" + ")
        }
        crate::spec::ProjectorSpec::Identity { axis } => {
            field_names.get(*axis).map(|n| n.as_str()).unwrap_or("0").to_string()
        }
        crate::spec::ProjectorSpec::Parity { axis } => {
            let name = field_names.get(*axis).map(|n| n.as_str()).unwrap_or("0");
            format!("{}[0]", name)
        }
    }
}

/// Generate a SystemVerilog assertion for a constraint.
fn sv_constraint_assertion(
    constraint: &crate::spec::ConstraintSpec,
    field_names: &[&String],
) -> String {
    match constraint {
        crate::spec::ConstraintSpec::Range { axis, min, max } => {
            let name = field_names.get(*axis).map(|n| n.as_str()).unwrap_or("0");
            format!(
                "assert property ({} >= {} && {} <= {}); // range\n",
                name, min, name, max
            )
        }
        crate::spec::ConstraintSpec::Even { axis } => {
            let name = field_names.get(*axis).map(|n| n.as_str()).unwrap_or("0");
            format!("assert property ({}[0] == 1'b0); // even\n", name)
        }
        crate::spec::ConstraintSpec::Eq { axis_a, axis_b } => {
            let name_a = field_names.get(*axis_a).map(|n| n.as_str()).unwrap_or("0");
            let name_b = field_names.get(*axis_b).map(|n| n.as_str()).unwrap_or("0");
            format!("assert property ({} == {}); // eq\n", name_a, name_b)
        }
    }
}

/// Run the default synthesis pipeline: SvGenerator + ScriptBackend (Yosys).
pub fn synthesize_default(spec: &VerificationSpec) -> anyhow::Result<SynthesisMetrics> {
    let script_path = if let Ok(path) = std::env::var("EV_SYNTH_SCRIPT") {
        std::path::PathBuf::from(path)
    } else {
        let candidates = [
                std::env::current_exe().ok().and_then(|p| {
                    p.parent().map(|p| p.join("../scripts/synth/default-synth.py"))
                }),
                Some(std::path::PathBuf::from("scripts/synth/default-synth.py")),
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
                     scripts/synth/default-synth.py"
                )
            })?
    };
    // Compose: SvGenerator (GenerateRtl) + backend (RunSynthesis).
    // When EV_SYNTH_BACKEND=mock, use MockSynthesisBackend for CI/testing.
    let rtl_path = SvGenerator.generate(spec)?;
    if std::env::var("EV_SYNTH_BACKEND").unwrap_or_default() == "mock" {
        MockSynthesisBackend.run(&rtl_path, &spec.target)
    } else {
        ScriptBackend::new(script_path).run(&rtl_path, &spec.target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::{ConstraintSpec, FieldSpec, ProjectorSpec, VerificationSpec};
    use std::collections::BTreeMap;

    fn make_spec(fields: BTreeMap<String, FieldSpec>, projector: ProjectorSpec) -> VerificationSpec {
        VerificationSpec {
            target: "test_module".into(),
            fields,
            constraints: vec![],
            projector,
        }
    }

    // ── field_bit_width ───────────────────────────────────────────

    #[test]
    fn bit_width_range_0_15() {
        let f = FieldSpec { range: Some((0, 15)), alignment: None, values: None };
        assert_eq!(field_bit_width(&f), 4); // 15 = 1111 → 4 bits
    }

    #[test]
    fn bit_width_range_0_4() {
        let f = FieldSpec { range: Some((0, 4)), alignment: None, values: None };
        assert_eq!(field_bit_width(&f), 3); // 4 = 100 → 3 bits
    }

    #[test]
    fn bit_width_range_0_0() {
        let f = FieldSpec { range: Some((0, 0)), alignment: None, values: None };
        assert_eq!(field_bit_width(&f), 1); // 0 → 1 bit minimum
    }

    #[test]
    fn bit_width_values_single_bit() {
        let f = FieldSpec { range: None, alignment: None, values: Some(vec![0, 1]) };
        assert_eq!(field_bit_width(&f), 1);
    }

    #[test]
    fn bit_width_values_three_bits() {
        let f = FieldSpec { range: None, alignment: None, values: Some(vec![0, 3, 7]) };
        assert_eq!(field_bit_width(&f), 3); // 7 = 111
    }

    #[test]
    fn bit_width_empty_values_defaults_to_1() {
        let f = FieldSpec { range: None, alignment: None, values: Some(vec![]) };
        assert_eq!(field_bit_width(&f), 1);
    }

    #[test]
    fn bit_width_defaults_to_8() {
        let f = FieldSpec { range: None, alignment: None, values: None };
        assert_eq!(field_bit_width(&f), 8);
    }

    // ── sv_projector ──────────────────────────────────────────────

    fn names(s: &[&str]) -> Vec<String> {
        s.iter().map(|n| n.to_string()).collect()
    }

    fn name_refs(v: &[String]) -> Vec<&String> {
        v.iter().collect()
    }

    #[test]
    fn projector_sum_two_fields() {
        let n = names(&["a", "b"]);
        let refs = name_refs(&n);
        let expr = sv_projector(&ProjectorSpec::Sum, &refs);
        assert_eq!(expr, "a + b");
    }

    #[test]
    fn projector_sum_single_field() {
        let n = names(&["x"]);
        let refs = name_refs(&n);
        let expr = sv_projector(&ProjectorSpec::Sum, &refs);
        assert_eq!(expr, "x");
    }

    #[test]
    fn projector_identity_axis_0() {
        let n = names(&["a", "b"]);
        let refs = name_refs(&n);
        let expr = sv_projector(&ProjectorSpec::Identity { axis: 0 }, &refs);
        assert_eq!(expr, "a");
    }

    #[test]
    fn projector_identity_axis_1() {
        let n = names(&["a", "b"]);
        let refs = name_refs(&n);
        let expr = sv_projector(&ProjectorSpec::Identity { axis: 1 }, &refs);
        assert_eq!(expr, "b");
    }

    #[test]
    fn projector_identity_out_of_bounds() {
        let n = names(&["a"]);
        let refs = name_refs(&n);
        let expr = sv_projector(&ProjectorSpec::Identity { axis: 5 }, &refs);
        assert_eq!(expr, "0");
    }

    #[test]
    fn projector_parity_extracts_lsb() {
        let n = names(&["x"]);
        let refs = name_refs(&n);
        let expr = sv_projector(&ProjectorSpec::Parity { axis: 0 }, &refs);
        assert_eq!(expr, "x[0]");
    }

    // ── sv_constraint_assertion ───────────────────────────────────

    #[test]
    fn constraint_range() {
        let n = names(&["a"]);
        let refs = name_refs(&n);
        let s = sv_constraint_assertion(&ConstraintSpec::Range { axis: 0, min: 0, max: 15 }, &refs);
        assert!(s.contains("assert property"));
        assert!(s.contains("a >= 0"));
        assert!(s.contains("a <= 15"));
    }

    #[test]
    fn constraint_even() {
        let n = names(&["x"]);
        let refs = name_refs(&n);
        let s = sv_constraint_assertion(&ConstraintSpec::Even { axis: 0 }, &refs);
        assert!(s.contains("assert property"));
        assert!(s.contains("x[0] == 1'b0"));
    }

    #[test]
    fn constraint_eq() {
        let n = names(&["a", "b"]);
        let refs = name_refs(&n);
        let s = sv_constraint_assertion(&ConstraintSpec::Eq { axis_a: 0, axis_b: 1 }, &refs);
        assert!(s.contains("assert property"));
        assert!(s.contains("a == b"));
    }

    // ── generate_sv ───────────────────────────────────────────────

    #[test]
    fn generate_sv_produces_module_header() {
        let mut fields = BTreeMap::new();
        fields.insert("op".into(), FieldSpec { range: Some((0, 7)), alignment: None, values: None });
        let spec = make_spec(fields, ProjectorSpec::Identity { axis: 0 });

        let sv_path = generate_sv(&spec).unwrap();
        let content = std::fs::read_to_string(&sv_path).unwrap();

        assert!(content.contains("module test_module ("));
        assert!(content.contains("input logic [2:0] op"));
        assert!(content.contains("output logic [2:0] result"));
        assert!(content.contains("assign result = op;"));
        assert!(content.contains("endmodule"));
    }

    #[test]
    fn generate_sv_with_constraints() {
        let mut fields = BTreeMap::new();
        fields.insert("a".into(), FieldSpec { range: Some((0, 15)), alignment: None, values: None });
        fields.insert("b".into(), FieldSpec { range: Some((0, 15)), alignment: None, values: None });
        let mut spec = make_spec(fields, ProjectorSpec::Sum);
        spec.constraints = vec![ConstraintSpec::Eq { axis_a: 0, axis_b: 1 }];

        let sv_path = generate_sv(&spec).unwrap();
        let content = std::fs::read_to_string(&sv_path).unwrap();

        assert!(content.contains("assign result = a + b;"));
        assert!(content.contains("assert property (a == b);"));
    }

    #[test]
    fn generate_sv_empty_fields_is_error() {
        let spec = make_spec(BTreeMap::new(), ProjectorSpec::Sum);
        let result = generate_sv(&spec);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("no fields"));
    }

    // ── SynthesisMetrics → Fact ────────────────────────────────────

    #[test]
    fn synthesis_metrics_to_fact() {
        let metrics = SynthesisMetrics {
            tool: "yosys".into(),
            version: "0.50".into(),
            source: "/tmp/test.sv".into(),
            module_name: "my_alu".into(),
            gate_count: Some(142),
            cell_area: Some(0.012),
            netlist_path: Some("/tmp/netlist.v".into()),
            dot_path: Some("/tmp/netlist.dot".into()),
            cell_types: None,
            warnings: None,
            status: "ok".into(),
            message: None,
        };
        let fact: crate::fih::Fact = metrics.into();

        assert_eq!(fact.fact_type, "synthesis_result");
        assert_eq!(fact.origin, "ev/synthesis");
        assert_eq!(fact.target, "my_alu");
        assert_eq!(fact.payload["tool"], "yosys");
        assert_eq!(fact.payload["gate_count"], 142);
        assert_eq!(fact.payload["netlist_path"], "/tmp/netlist.v");
        assert_eq!(fact.payload["dot_path"], "/tmp/netlist.dot");
        assert_eq!(fact.payload["status"], "ok");
        assert!(!fact.timestamp.is_empty());
    }

    #[test]
    fn synthesis_metrics_error_to_fact() {
        let metrics = SynthesisMetrics {
            tool: "unknown".into(),
            version: "unknown".into(),
            source: "/tmp/test.sv".into(),
            module_name: "bad_module".into(),
            gate_count: None,
            cell_area: None,
            netlist_path: None,
            dot_path: None,
            cell_types: None,
            warnings: None,
            status: "error".into(),
            message: Some("yosys not found".into()),
        };
        let fact: crate::fih::Fact = metrics.into();

        assert_eq!(fact.fact_type, "synthesis_result");
        assert_eq!(fact.payload["status"], "error");
        assert_eq!(fact.payload["message"], "yosys not found");
        assert!(fact.payload["gate_count"].is_null());
    }

    // ── MockSynthesisBackend ───────────────────────────────────────

    #[test]
    fn mock_backend_valid_sv() {
        let mut fields = BTreeMap::new();
        fields.insert("a".into(), FieldSpec { range: Some((0, 7)), alignment: None, values: None });
        let spec = make_spec(fields, ProjectorSpec::Identity { axis: 0 });
        let rtl_path = SvGenerator.generate(&spec).unwrap();

        let metrics = MockSynthesisBackend.run(&rtl_path, "test_module").unwrap();
        assert_eq!(metrics.tool, "mock");
        assert_eq!(metrics.status, "ok");
        assert_eq!(metrics.gate_count, Some(0));
    }

    #[test]
    fn mock_backend_wrong_module_name() {
        let mut fields = BTreeMap::new();
        fields.insert("a".into(), FieldSpec { range: Some((0, 7)), alignment: None, values: None });
        let spec = make_spec(fields, ProjectorSpec::Identity { axis: 0 });
        let rtl_path = SvGenerator.generate(&spec).unwrap();

        let result = MockSynthesisBackend.run(&rtl_path, "nonexistent_module");
        assert!(result.is_err());
    }

    #[test]
    fn mock_backend_missing_file() {
        let result = MockSynthesisBackend.run(
            std::path::Path::new("/tmp/ev-nonexistent.sv"),
            "foo",
        );
        assert!(result.is_err());
    }

    #[test]
    fn full_pipeline_with_mock() {
        let mut fields = BTreeMap::new();
        fields.insert("x".into(), FieldSpec { range: Some((0, 3)), alignment: None, values: None });
        let spec = make_spec(fields, ProjectorSpec::Sum);

        let rtl_path = SvGenerator.generate(&spec).unwrap();
        let metrics = MockSynthesisBackend.run(&rtl_path, "test_module").unwrap();

        assert_eq!(metrics.module_name, "test_module");
        assert_eq!(metrics.status, "ok");
    }
}
