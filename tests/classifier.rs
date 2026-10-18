//! The classification capability: the built-in structural engine and the
//! naive reference produce comparable records, and each record names its
//! engine and claim class.

use ev::classification::{ClaimClass, Classification, EngineIdentity};
use ev::spec::VerificationSpec;
use ev::verify::{
    Classify, ConstraintRegistry, NaiveClassifier, ProjectorRegistry, StructuralClassifier,
};
use std::collections::BTreeMap;
use std::path::Path;

fn load(path: &str) -> VerificationSpec {
    VerificationSpec::from_yaml(Path::new(path))
        .unwrap_or_else(|error| panic!("failed to load {path}: {error}"))
}

fn outcomes(classification: &Classification) -> BTreeMap<Vec<i64>, (bool, Option<i64>)> {
    classification
        .verdicts
        .iter()
        .map(|verdict| (verdict.values.clone(), (verdict.passed, verdict.projection)))
        .collect()
}

fn run<T: Classify>(engine: &T, spec: &VerificationSpec) -> Classification {
    engine
        .classify(
            spec,
            &ConstraintRegistry::default(),
            &ProjectorRegistry::default(),
        )
        .expect("classification succeeds")
}

#[test]
fn records_name_the_engine_and_claim() {
    let spec = load("tests/fixtures/common/sample.xif.yaml");
    let structural = run(&StructuralClassifier, &spec);
    let naive = run(&NaiveClassifier, &spec);

    assert_eq!(structural.engine.name, "ev/structural");
    assert_eq!(naive.engine.name, "ev/naive");
    assert_eq!(structural.claim, ClaimClass::Exhaustive);
    assert_eq!(naive.claim, ClaimClass::Exhaustive);
}

#[test]
fn the_engines_agree_on_a_runtime_only_space() {
    let spec = load("tests/fixtures/common/sample.xif.yaml");
    let structural = run(&StructuralClassifier, &spec);
    let naive = run(&NaiveClassifier, &spec);

    assert_eq!(structural.field_order, naive.field_order);
    assert_eq!(structural.total, naive.total);
    assert_eq!(outcomes(&structural), outcomes(&naive));
}

#[cfg(feature = "riscv")]
#[test]
fn the_engines_agree_at_each_emitted_coordinate() {
    let spec = load("tests/fixtures/cva6/xif_ref_r4.xif.yaml");
    let structural = run(&StructuralClassifier, &spec);
    let naive = run(&NaiveClassifier, &spec);

    assert_eq!(structural.field_order, naive.field_order);
    assert_eq!(structural.total, naive.total);

    let structural_outcomes = outcomes(&structural);
    let naive_outcomes = outcomes(&naive);

    for (values, result) in &structural_outcomes {
        assert_eq!(
            naive_outcomes.get(values),
            Some(result),
            "the engines disagree at {values:?}"
        );
    }
    for (values, (passed, _)) in &naive_outcomes {
        if !structural_outcomes.contains_key(values) {
            assert!(
                !passed,
                "the structural engine omitted a passing point {values:?}"
            );
        }
    }
}

#[test]
fn a_partial_claim_is_distinct_from_an_exhaustive_one() {
    let exhaustive = Classification::new(Vec::new(), 0, Vec::new());
    let partial = Classification::by(
        EngineIdentity::new("test/partial", "0"),
        ClaimClass::Partial,
        Vec::new(),
        0,
        Vec::new(),
    );

    assert_eq!(
        serde_json::to_string(&exhaustive.claim).unwrap(),
        "\"exhaustive\""
    );
    assert_eq!(
        serde_json::to_string(&partial.claim).unwrap(),
        "\"partial\""
    );
}
