//! Reporter capability — formats verification results for output.
//!
//! Following the Nexus capability-trait pattern: each output format (text, JSON)
//! implements this trait. The pipeline only depends on the trait.

use crate::evaluate::Evaluation;
use std::collections::BTreeMap;

/// Capability: format and output verification results.
pub trait ReporterCapable: Send + Sync {
    /// Report results. Returns true if all evaluations passed.
    fn report(&self, target: &str, field_order: &[String], evaluations: &[Evaluation]) -> bool;
}

// ============================================================================
// Text Reporter
// ============================================================================

pub struct TextReporter;

impl ReporterCapable for TextReporter {
    fn report(&self, target: &str, _field_order: &[String], evaluations: &[Evaluation]) -> bool {
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

#[derive(Debug, Serialize)]
struct EvaluationEntry {
    combination: Vec<i64>,
    fields: BTreeMap<String, i64>,
    passed: bool,
    projection: Option<i64>,
    #[serde(skip_serializing_if = "String::is_empty")]
    reason: String,
}

#[derive(Debug, Serialize)]
struct VerificationReport {
    target: String,
    total: usize,
    passed: usize,
    failed: usize,
    field_order: Vec<String>,
    results: Vec<EvaluationEntry>,
}

pub struct JsonReporter;

impl ReporterCapable for JsonReporter {
    fn report(&self, target: &str, field_order: &[String], evaluations: &[Evaluation]) -> bool {
        let passed_count = evaluations.iter().filter(|e| e.passed).count();
        let failed_count = evaluations.len() - passed_count;

        let report = VerificationReport {
            target: target.to_string(),
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
                    EvaluationEntry {
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
