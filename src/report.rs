//! Output formatting — text and JSON reporting of verification results.

use crate::evaluate::Evaluation;
use serde::Serialize;

/// Serializable representation of a single evaluation result.
#[derive(Debug, Serialize)]
struct EvaluationEntry {
    combination: Vec<i64>,
    fields: std::collections::BTreeMap<String, i64>,
    passed: bool,
    projection: Option<i64>,
    #[serde(skip_serializing_if = "String::is_empty")]
    reason: String,
}

/// Serializable summary of all verification results.
#[derive(Debug, Serialize)]
struct VerificationReport {
    target: String,
    total: usize,
    passed: usize,
    failed: usize,
    field_order: Vec<String>,
    results: Vec<EvaluationEntry>,
}

/// Print results as human-readable text to stdout.
///
/// Returns true if all evaluations passed.
pub fn report_text(target: &str, evaluations: &[Evaluation]) -> bool {
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

/// Print results as JSON to stdout.
///
/// Returns true if all evaluations passed.
pub fn report_json(target: &str, field_names: &[&str], evaluations: &[Evaluation]) -> bool {
    let passed_count = evaluations.iter().filter(|e| e.passed).count();
    let failed_count = evaluations.len() - passed_count;

    let report = VerificationReport {
        target: target.to_string(),
        total: evaluations.len(),
        passed: passed_count,
        failed: failed_count,
        field_order: field_names.iter().map(|s| s.to_string()).collect(),
        results: evaluations
            .iter()
            .map(|e| {
                let fields: std::collections::BTreeMap<String, i64> = field_names
                    .iter()
                    .enumerate()
                    .filter_map(|(i, name)| {
                        e.combination.values.get(i).map(|v| (name.to_string(), *v))
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
