//! Reporter capability — formats verification results for output.
//!
//! Following the Nexus capability-trait pattern: each output format (text, JSON,
//! CSV, trace) implements this trait. The pipeline only depends on the trait.
//!
//! Note: the trait is deliberately minimal (`target`, `spec_hash`, `field_order`,
//! `evaluations`). It does NOT depend on `VerificationSpec` so that any colony
//! — not just ev — can implement it.

use crate::verify::evaluate::Evaluation;
use sha2::{Digest, Sha256};

/// Capability: format and output verification results.
///
/// Takes only the data it needs (target name, optional spec hash for content-
/// addressing, field order, and evaluations). Does NOT take `&VerificationSpec`
/// to keep the trait reusable across colonies.
pub trait ReporterCapable: Send + Sync {
    /// Report results. Returns true if all evaluations passed.
    ///
    /// * `target` — human-readable name of the verified target.
    /// * `spec_hash` — content-addressable hash of the spec (empty string if
    ///   not available / not needed, e.g. text reporter).
    /// * `field_order` — ordered field names matching evaluation values.
    /// * `evaluations` — individual evaluation results.
    fn report(
        &self,
        target: &str,
        spec_hash: &str,
        field_order: &[String],
        evaluations: &[Evaluation],
    ) -> bool;
}

// ============================================================================
// CSV Reporter
// ============================================================================

pub struct CsvReporter;

impl ReporterCapable for CsvReporter {
    fn report(
        &self,
        target: &str,
        _spec_hash: &str,
        field_order: &[String],
        evaluations: &[Evaluation],
    ) -> bool {
        let passed_count = evaluations.iter().filter(|e| e.passed).count();
        let failed_count = evaluations.len() - passed_count;

        // Print metadata as comments
        println!("# target: {}", target);
        println!("# total:  {}", evaluations.len());
        println!("# passed: {}", passed_count);
        println!("# failed: {}", failed_count);
        println!();

        // Header
        let mut header = field_order.join(",");
        header.push_str(",passed,projection");
        println!("{}", header);

        // Rows — use field_order to align values with header
        for e in evaluations {
            let values: Vec<String> = field_order
                .iter()
                .enumerate()
                .map(|(i, _name)| {
                    e.combination
                        .values
                        .get(i)
                        .map(|v| v.to_string())
                        .unwrap_or_default()
                })
                .collect();
            let mut row = values.join(",");
            row.push(',');
            row.push_str(if e.passed { "true" } else { "false" });
            row.push(',');
            match e.projection {
                Some(p) => row.push_str(&p.to_string()),
                None => row.push_str("N/A"),
            }
            println!("{}", row);
        }

        failed_count == 0
    }
}

// ============================================================================
// Trace Reporter
// ============================================================================

pub struct TraceReporter;

impl ReporterCapable for TraceReporter {
    fn report(
        &self,
        target: &str,
        _spec_hash: &str,
        field_order: &[String],
        evaluations: &[Evaluation],
    ) -> bool {
        let passed_count = evaluations.iter().filter(|e| e.passed).count();
        let failed_count = evaluations.len() - passed_count;
        let started_at = chrono::Utc::now();

        println!(
            "[{}] TRACE  verification_started  target={}",
            started_at.to_rfc3339(),
            target
        );
        println!(
            "[{}] INFO   total_combinations={}",
            started_at.to_rfc3339(),
            evaluations.len()
        );
        println!();

        for e in evaluations.iter() {
            let ts = chrono::Utc::now();
            let values: String = field_order
                .iter()
                .enumerate()
                .map(|(j, name)| {
                    let v = e
                        .combination
                        .values
                        .get(j)
                        .map(|v| v.to_string())
                        .unwrap_or_default();
                    format!("{}={}", name, v)
                })
                .collect::<Vec<_>>()
                .join(" ");
            let status = if e.passed { "PASS" } else { "FAIL" };
            let projection = match e.projection {
                Some(p) => format!(" projection={}", p),
                None => String::new(),
            };
            let reason = if !e.reason.is_empty() {
                format!(" reason={}", e.reason)
            } else {
                String::new()
            };
            println!(
                "[{}] {}   target={} values=({}){}{}",
                ts.to_rfc3339(),
                status,
                target,
                values,
                projection,
                reason
            );
        }

        println!();
        let finished_at = chrono::Utc::now();
        println!(
            "[{}] TRACE  verification_finished  passed={} failed={} total={}",
            finished_at.to_rfc3339(),
            passed_count,
            failed_count,
            evaluations.len()
        );

        failed_count == 0
    }
}

// ============================================================================
// Text Reporter
// ============================================================================

pub struct TextReporter;

impl ReporterCapable for TextReporter {
    fn report(
        &self,
        target: &str,
        _spec_hash: &str,
        _field_order: &[String],
        evaluations: &[Evaluation],
    ) -> bool {
        let passed_count = evaluations.iter().filter(|e| e.passed).count();
        let failed_count = evaluations.len() - passed_count;

        println!("target: {}", target);
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
use std::collections::BTreeMap;

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
        target: &str,
        spec_hash: &str,
        field_order: &[String],
        evaluations: &[Evaluation],
    ) -> bool {
        let passed_count = evaluations.iter().filter(|e| e.passed).count();
        let failed_count = evaluations.len() - passed_count;
        let origin = format!("ev/{}", env!("CARGO_PKG_VERSION"));
        let timestamp = chrono::Utc::now().to_rfc3339();
        let spec_hash = spec_hash.to_string();

        let report = VerificationReport {
            origin: origin.clone(),
            target: target.to_string(),
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

        let fact = crate::report::fih::Fact::new(
            "verification_result",
            &origin,
            target,
            serde_json::to_vec(&report).unwrap_or_default(),
        );
        // CLI output: emit the full Fact envelope for machine consumption.
        let json = serde_json::to_string_pretty(&fact).unwrap_or_else(|_| "{}".to_string());
        println!("{}", json);

        failed_count == 0
    }
}

// ── Content-addressable hashing ──────────────────────────────────────────
// Public so callers (e.g. main.rs) can compute the hash and pass it to
// ReporterCapable::report.

/// Hash a spec into a deterministic content ID.
///
/// Public so the caller can compute this once and pass to the reporter
/// trait, keeping the trait free of `&VerificationSpec` dependencies.
pub fn hash_spec(spec: &crate::spec::VerificationSpec) -> String {
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

/// Hash all evaluations into a single content ID for simulation results.
#[allow(dead_code)]
pub fn hash_evaluations(evaluations: &[Evaluation]) -> String {
    let mut h = Sha256::new();
    for e in evaluations {
        h.update(e.combination.values.len().to_le_bytes());
        for v in &e.combination.values {
            h.update(v.to_le_bytes());
        }
        h.update(if e.passed { b"1" } else { b"0" });
        h.update(e.reason.as_bytes());
    }
    format!("{:x}", h.finalize())
}

/// Hash a single evaluation result. Tied to its parent spec via spec_hash.
pub fn hash_evaluation(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::{ConstraintSpec, FieldSpec, ProjectorSpec, VerificationSpec};
    use std::collections::BTreeMap;

    fn make_spec() -> VerificationSpec {
        let mut fields = BTreeMap::new();
        fields.insert(
            "op".into(),
            FieldSpec {
                range: Some((0, 7)),
                alignment: None,
                values: None,
            },
        );
        fields.insert(
            "rs1".into(),
            FieldSpec {
                range: Some((0, 31)),
                alignment: None,
                values: None,
            },
        );
        VerificationSpec {
            target: "test".into(),
            fields,
            encoding: None,
            constraints: vec![ConstraintSpec::Eq {
                field_a: "op".into(),
                field_b: "rs1".into(),
            }],
            projector: ProjectorSpec::Sum,
        }
    }

    #[test]
    fn hash_spec_deterministic() {
        let spec = make_spec();
        let h1 = hash_spec(&spec);
        let h2 = hash_spec(&spec);
        assert_eq!(h1, h2, "same spec should produce same hash");
        assert_eq!(h1.len(), 64, "SHA256 hex should be 64 chars");
    }

    #[test]
    fn hash_spec_differs_when_fields_change() {
        let spec1 = make_spec();
        let mut spec2 = make_spec();
        spec2.target = "other".into();
        let h1 = hash_spec(&spec1);
        let h2 = hash_spec(&spec2);
        assert_ne!(h1, h2, "different target should produce different hash");
    }

    #[test]
    fn hash_evaluation_deterministic() {
        let spec = make_spec();
        let spec_hash = hash_spec(&spec);
        let values = [1, 2, 3];

        let h1 = hash_evaluation(&spec_hash, &values, true, Some(3));
        let h2 = hash_evaluation(&spec_hash, &values, true, Some(3));
        assert_eq!(h1, h2, "same inputs should produce same hash");
    }

    #[test]
    fn hash_evaluation_passed_failed_differ() {
        let spec = make_spec();
        let spec_hash = hash_spec(&spec);
        let values = [5, 5, 5];

        let h_pass = hash_evaluation(&spec_hash, &values, true, Some(10));
        let h_fail = hash_evaluation(&spec_hash, &values, false, Some(10));
        assert_ne!(
            h_pass, h_fail,
            "passed vs failed should produce different hashes"
        );
    }

    #[test]
    fn hash_evaluation_none_projection() {
        let spec = make_spec();
        let spec_hash = hash_spec(&spec);
        let values = [0, 0, 0];

        let h = hash_evaluation(&spec_hash, &values, true, None);
        assert_eq!(h.len(), 64, "SHA256 hex should be 64 chars");
    }
}
