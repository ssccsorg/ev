//! Reporter capability — formats verification results for output.
//!
//! Following the Nexus capability-trait pattern: each output format (text, JSON)
//! implements this trait. The pipeline only depends on the trait.

use crate::evaluate::Evaluation;
use crate::spec::VerificationSpec;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

/// Capability: format and output verification results.
pub trait ReporterCapable: Send + Sync {
    /// Report results. Returns true if all evaluations passed.
    fn report(
        &self,
        spec: &VerificationSpec,
        field_order: &[String],
        evaluations: &[Evaluation],
    ) -> bool;
}

// ============================================================================
// Text Reporter
// ============================================================================

pub struct TextReporter;

impl ReporterCapable for TextReporter {
    fn report(
        &self,
        spec: &VerificationSpec,
        _field_order: &[String],
        evaluations: &[Evaluation],
    ) -> bool {
        let passed_count = evaluations.iter().filter(|e| e.passed).count();
        let failed_count = evaluations.len() - passed_count;

        println!("target: {}", spec.target);
        println!("total:  {}", evaluations.len());
        println!("passed: {}", passed_count);
        println!("failed: {}", failed_count);
        println!();

        if failed_count > 0 {
            println!("Failures:");
            for e in evaluations.iter().filter(|e| !e.passed) {
                println!("  [FAIL] {:?} — {}", e.combination.values, e.reason);
            }
        }

        if passed_count > 0 && failed_count == 0 {
            println!("All combinations passed.");
        }

        failed_count == 0
    }
}

// ============================================================================
// JSON Reporter
// ============================================================================

use serde::Serialize;

#[derive(Debug, Serialize)]
struct EvaluationEntry {
    /// Content-addressable ID: SHA256(spec_hash || values || passed || projection).
    id: String,
    combination: Vec<i64>,
    fields: BTreeMap<String, i64>,
    passed: bool,
    projection: Option<i64>,
    #[serde(skip_serializing_if = "String::is_empty")]
    reason: String,
}

#[derive(Debug, Serialize)]
struct VerificationReport {
    /// Origin: "ev/{version}".
    origin: String,
    /// Target module identifier.
    target: String,
    /// ISO 8601 timestamp of this verification run.
    timestamp: String,
    /// Content-addressable hash of the specification (parent reference).
    spec_hash: String,
    total: usize,
    passed: usize,
    failed: usize,
    field_order: Vec<String>,
    results: Vec<EvaluationEntry>,
}

pub struct JsonReporter;

impl ReporterCapable for JsonReporter {
    fn report(
        &self,
        spec: &VerificationSpec,
        field_order: &[String],
        evaluations: &[Evaluation],
    ) -> bool {
        let passed_count = evaluations.iter().filter(|e| e.passed).count();
        let failed_count = evaluations.len() - passed_count;
        let spec_hash = hash_spec(spec);
        let origin = format!("ev/{}", env!("CARGO_PKG_VERSION"));
        let timestamp = chrono::Utc::now().to_rfc3339();

        let report = VerificationReport {
            origin,
            target: spec.target.clone(),
            timestamp,
            spec_hash: spec_hash.clone(),
            total: evaluations.len(),
            passed: passed_count,
            failed: failed_count,
            field_order: field_order.to_vec(),
            results: evaluations
                .iter()
                .map(|e| {
                    let fields: BTreeMap<String, i64> = field_order
                        .iter()
                        .enumerate()
                        .filter_map(|(i, name)| {
                            e.combination.values.get(i).map(|v| (name.clone(), *v))
                        })
                        .collect();
                    let id =
                        hash_evaluation(&spec_hash, &e.combination.values, e.passed, e.projection);
                    EvaluationEntry {
                        id,
                        combination: e.combination.values.clone(),
                        fields,
                        passed: e.passed,
                        projection: e.projection,
                        reason: e.reason.clone(),
                    }
                })
                .collect(),
        };

        let json = serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string());
        println!("{}", json);

        failed_count == 0
    }
}

// ── Content-addressable hashing ──────────────────────────────────────────

/// Hash a spec into a deterministic content ID.
fn hash_spec(spec: &VerificationSpec) -> String {
    let mut h = Sha256::new();
    h.update(spec.target.as_bytes());
    for (name, field) in &spec.fields {
        h.update(name.as_bytes());
        h.update(format!("{:?}", field.range).as_bytes());
        h.update(format!("{:?}", field.alignment).as_bytes());
        h.update(format!("{:?}", field.values).as_bytes());
    }
    h.update(format!("{:?}", spec.projector).as_bytes());
    for c in &spec.constraints {
        h.update(format!("{:?}", c).as_bytes());
    }
    format!("{:x}", h.finalize())
}

/// Hash a single evaluation result. Tied to its parent spec via spec_hash.
fn hash_evaluation(
    spec_hash: &str,
    values: &[i64],
    passed: bool,
    projection: Option<i64>,
) -> String {
    let mut h = Sha256::new();
    h.update(spec_hash.as_bytes());
    for v in values {
        h.update(v.to_le_bytes());
    }
    h.update(if passed { b"1" } else { b"0" });
    if let Some(p) = projection {
        h.update(p.to_le_bytes());
    }
    format!("{:x}", h.finalize())
}
