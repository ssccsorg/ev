// Temporary ibex timing probe; removed after use.
use ev::spec::VerificationSpec;
use ev::verify::compose::{expand_all, StructuralEnum};
use ev::verify::evaluate::evaluate_all;
use ev::verify::registry::{Check, ConstraintRegistry, ProjectorRegistry};
use std::time::Instant;

fn main() {
    let spec = VerificationSpec::from_yaml(std::path::Path::new(
        "tests/fixtures/ibex/rv32imcb.xif.yaml",
    ))
    .unwrap();
    let combos = expand_all(&spec).unwrap();

    // (a) Time only the cross check over all combinations.
    let checks = ConstraintRegistry::default().build_all(&spec.constraints, &spec.fields);
    let t = Instant::now();
    let mut pass = 0usize;
    for c in &combos {
        if checks.iter().all(|ck| ck.allows(c.point.coordinates())) {
            pass += 1;
        }
    }
    println!("checks only: {:?} pass={}", t.elapsed(), pass);

    let t = Instant::now();
    let results = evaluate_all(
        &spec,
        combos,
        &ConstraintRegistry::default(),
        &ProjectorRegistry::default(),
    );
    let passed = results.iter().filter(|r| r.passed).count();
    println!("evaluate_all: {:?} passed={}", t.elapsed(), passed);
    let t = Instant::now();
    let n = StructuralEnum::new(&spec).count();
    println!("struct_enum: {:?} count={}", t.elapsed(), n);
}
