// Temporary timing reproduction for the CVA6 XIF fixtures.
// Not part of the deliverable; removed after verification.
use ev::spec::VerificationSpec;
use ev::verify::compose::{expand_all, StructuralEnum};
use ev::verify::evaluate::evaluate_all;
use ev::verify::registry::{ConstraintRegistry, ProjectorRegistry};
use std::time::Instant;

fn main() {
    for (name, path) in [
        ("cva6_r4", "tests/fixtures/cva6/xif_ref_r4.xif.yaml"),
        ("cva6_full", "tests/fixtures/cva6/xif_ref.xif.yaml"),
    ] {
        let spec = VerificationSpec::from_yaml(std::path::Path::new(path)).unwrap();

        let t = Instant::now();
        let n = StructuralEnum::new(&spec).count();
        println!("{name} struct_enum: {:?} count={}", t.elapsed(), n);

        let t = Instant::now();
        let combos = expand_all(&spec).unwrap();
        println!("{name} expand_all: {:?} combos={}", t.elapsed(), combos.len());

        let t = Instant::now();
        let results = evaluate_all(
            &spec,
            combos,
            &ConstraintRegistry::default(),
            &ProjectorRegistry::default(),
        );
        let passed = results.iter().filter(|r| r.passed).count();
        println!(
            "{name} evaluate_all: {:?} passed={}",
            t.elapsed(),
            passed
        );
    }
}
