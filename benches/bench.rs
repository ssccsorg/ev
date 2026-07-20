//! ev benchmarks — exhaustive verification pipeline performance.
//!
//! Measures domain expansion, constraint evaluation, and CoordSpace-based
//! verification against the standard Vec-based pipeline.
//!
//! Run: cargo bench
//! Filter: cargo bench -- expand  (runs only expand_* benchmarks)

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::collections::BTreeMap;

use ev::spec::{ConstraintSpec, FieldSpec, ProjectorSpec, VerificationSpec};
use ev::verify::compose::{coords_to_coord_vec, expand_all, EnumerateIter, StructuralEnum};
use ev::verify::evaluate::{evaluate_all, validate_into_space};
use ev::verify::registry::{Check, ConstraintRegistry, ProjectorRegistry};

// ===========================================================================
// Helpers
// ===========================================================================

/// Small spec: 3 fields × 3 values = 27 combos.
fn small_spec() -> VerificationSpec {
    let mut fields = BTreeMap::new();
    fields.insert(
        "a".into(),
        FieldSpec {
            range: Some((0, 2)),
            alignment: None,
            values: None,
        },
    );
    fields.insert(
        "b".into(),
        FieldSpec {
            range: Some((0, 2)),
            alignment: None,
            values: None,
        },
    );
    fields.insert(
        "c".into(),
        FieldSpec {
            range: Some((0, 2)),
            alignment: None,
            values: None,
        },
    );
    VerificationSpec {
        target: "bench_small".into(),
        fields,
        encoding: None,
        constraints: vec![],
        projector: ProjectorSpec::Sum,
    }
}

/// Medium spec: 5 fields × 8 values = 32,768 combos (ibex-like).
fn medium_spec() -> VerificationSpec {
    let mut fields = BTreeMap::new();
    fields.insert(
        "funct7".into(),
        FieldSpec {
            range: Some((0, 7)),
            alignment: None,
            values: None,
        },
    );
    fields.insert(
        "funct3".into(),
        FieldSpec {
            range: Some((0, 7)),
            alignment: None,
            values: None,
        },
    );
    fields.insert(
        "rs1".into(),
        FieldSpec {
            range: Some((0, 7)),
            alignment: None,
            values: None,
        },
    );
    fields.insert(
        "rs2".into(),
        FieldSpec {
            range: Some((0, 7)),
            alignment: None,
            values: None,
        },
    );
    fields.insert(
        "rd".into(),
        FieldSpec {
            range: Some((0, 7)),
            alignment: None,
            values: None,
        },
    );
    let mapping: std::collections::HashMap<i64, Vec<i64>> = [
        (0, vec![0, 1, 2, 3, 4, 5, 6, 7]),
        (4, vec![1, 4, 5, 6, 7]),
        (5, vec![1, 2, 3, 4, 5, 6, 7]),
    ]
    .into();
    VerificationSpec {
        target: "bench_medium".into(),
        fields,
        encoding: None,
        constraints: vec![ConstraintSpec::Cross {
            field_a: "funct7".into(),
            field_b: "funct3".into(),
            mapping,
        }],
        projector: ProjectorSpec::Sum,
    }
}

/// Ibex rv32imcb spec (524,288 combos, Zbt + main case).
fn ibex_spec() -> VerificationSpec {
    let path = std::path::Path::new("tests/fixtures/ibex/rv32imcb.xif.yaml");
    VerificationSpec::from_yaml(path).expect("failed to load rv32imcb fixture")
}

/// CVA6 XIF ref r4 spec (2,097,152 combos).
fn cva6_r4_spec() -> VerificationSpec {
    let path = std::path::Path::new("tests/fixtures/cva6/xif_ref_r4.xif.yaml");
    VerificationSpec::from_yaml(path).expect("failed to load cva6 xif ref r4 fixture")
}

fn cva6_full_spec() -> VerificationSpec {
    let path = std::path::Path::new("tests/fixtures/cva6/xif_ref.xif.yaml");
    VerificationSpec::from_yaml(path).expect("failed to load cva6 xif ref fixture")
}

// ===========================================================================
// Domain expansion: expand_all (Vec) vs EnumerateIter
// ===========================================================================

fn bench_expand_small(c: &mut Criterion) {
    let spec = small_spec();
    c.bench_function("expand/small_27", |b| {
        b.iter(|| {
            let combos = expand_all(black_box(&spec)).unwrap();
            black_box(combos);
        })
    });
}

fn bench_enumerate_small(c: &mut Criterion) {
    let spec = small_spec();
    c.bench_function("enumerate/small_27", |b| {
        b.iter(|| {
            let iter = EnumerateIter::new(black_box(&spec));
            let count = iter.count();
            black_box(count);
        })
    });
}

fn bench_expand_medium(c: &mut Criterion) {
    let spec = medium_spec();
    c.bench_function("expand/medium_32k", |b| {
        b.iter(|| {
            let combos = expand_all(black_box(&spec)).unwrap();
            black_box(combos);
        })
    });
}

fn bench_enumerate_medium(c: &mut Criterion) {
    let spec = medium_spec();
    c.bench_function("enumerate/medium_32k", |b| {
        b.iter(|| {
            let iter = EnumerateIter::new(black_box(&spec));
            let count = iter.count();
            black_box(count);
        })
    });
}

fn bench_expand_ibex(c: &mut Criterion) {
    let spec = ibex_spec();
    c.bench_function("expand/ibex_524k", |b| {
        b.iter(|| {
            let combos = expand_all(black_box(&spec)).unwrap();
            black_box(combos);
        })
    });
}

fn bench_enumerate_ibex(c: &mut Criterion) {
    let spec = ibex_spec();
    c.bench_function("enumerate/ibex_524k", |b| {
        b.iter(|| {
            let iter = EnumerateIter::new(black_box(&spec));
            let count = iter.count();
            black_box(count);
        })
    });
}

// ===========================================================================
// Evaluation: evaluate_all (Vec) vs validate_into_space (CoordSpace)
// ===========================================================================

fn bench_evaluate_small(c: &mut Criterion) {
    let spec = small_spec();
    let combos = expand_all(&spec).unwrap();
    c.bench_function("evaluate/small_27", |b| {
        b.iter(|| {
            let results = evaluate_all(
                black_box(&spec),
                black_box(combos.clone()),
                &ConstraintRegistry::default(),
                &ProjectorRegistry::default(),
            );
            black_box(results);
        })
    });
}

fn bench_structural_enum_small(c: &mut Criterion) {
    let spec = small_spec();
    c.bench_function("struct_enum/small_27", |b| {
        b.iter(|| {
            let count = StructuralEnum::new(black_box(&spec)).count();
            black_box(count);
        })
    });
}

fn bench_validate_small(c: &mut Criterion) {
    let spec = small_spec();
    c.bench_function("validate/small_27", |b| {
        b.iter(|| {
            let space = validate_into_space(black_box(&spec), &ConstraintRegistry::default());
            black_box(space);
        })
    });
}

fn bench_evaluate_medium(c: &mut Criterion) {
    let spec = medium_spec();
    let combos = expand_all(&spec).unwrap();
    c.bench_function("evaluate/medium_32k", |b| {
        b.iter(|| {
            let results = evaluate_all(
                black_box(&spec),
                black_box(combos.clone()),
                &ConstraintRegistry::default(),
                &ProjectorRegistry::default(),
            );
            black_box(results);
        })
    });
}

fn bench_structural_enum_medium(c: &mut Criterion) {
    let spec = medium_spec();
    c.bench_function("struct_enum/medium_32k", |b| {
        b.iter(|| {
            let count = StructuralEnum::new(black_box(&spec)).count();
            black_box(count);
        })
    });
}

fn bench_validate_medium(c: &mut Criterion) {
    let spec = medium_spec();
    c.bench_function("validate/medium_32k", |b| {
        b.iter(|| {
            let space = validate_into_space(black_box(&spec), &ConstraintRegistry::default());
            black_box(space);
        })
    });
}

fn bench_evaluate_ibex(c: &mut Criterion) {
    let spec = ibex_spec();
    let combos = expand_all(&spec).unwrap();
    c.bench_function("evaluate/ibex_524k", |b| {
        b.iter(|| {
            let results = evaluate_all(
                black_box(&spec),
                black_box(combos.clone()),
                &ConstraintRegistry::default(),
                &ProjectorRegistry::default(),
            );
            let count = results.iter().filter(|r| r.passed).count();
            black_box(count);
        })
    });
}

fn bench_structural_enum_ibex(c: &mut Criterion) {
    let spec = ibex_spec();
    c.bench_function("struct_enum/ibex_524k", |b| {
        b.iter(|| {
            let count = StructuralEnum::new(black_box(&spec)).count();
            black_box(count);
        })
    });
}

fn bench_validate_ibex(c: &mut Criterion) {
    let spec = ibex_spec();
    c.bench_function("validate/ibex_524k", |b| {
        b.iter(|| {
            let space = validate_into_space(black_box(&spec), &ConstraintRegistry::default());
            let count = space.iter().count();
            black_box(count);
        })
    });
}

// ===========================================================================
// CoordSpace: lookup vs Vec scan for individual combination validity
// ===========================================================================

fn bench_space_lookup_valid(c: &mut Criterion) {
    let spec = ibex_spec();
    let space = validate_into_space(&spec, &ConstraintRegistry::default());
    // A known valid combination: funct7=4, funct3=4 (PACK), rs1=0, rs2=0, rd=0
    let valid_path = coords_to_coord_vec(&[4, 4, 0, 0, 0]).unwrap();
    c.bench_function("coordspace/lookup_valid", |b| {
        b.iter(|| {
            let result = space.at(black_box(&valid_path));
            black_box(result);
        })
    });
}

fn bench_space_lookup_invalid(c: &mut Criterion) {
    let spec = ibex_spec();
    let space = validate_into_space(&spec, &ConstraintRegistry::default());
    // A known invalid combination: funct7=4, funct3=0 (not in mapping)
    let invalid_path = coords_to_coord_vec(&[4, 0, 0, 0, 0]).unwrap();
    c.bench_function("coordspace/lookup_invalid", |b| {
        b.iter(|| {
            let result = space.at(black_box(&invalid_path));
            black_box(result);
        })
    });
}

// ===========================================================================
// Constraint check: single combination
// ===========================================================================

fn bench_expand_cva6_r4(c: &mut Criterion) {
    let spec = cva6_r4_spec();
    c.bench_function("expand/cva6_r4_2M", |b| {
        b.iter(|| {
            let combos = expand_all(black_box(&spec)).unwrap();
            black_box(combos);
        })
    });
}

fn bench_enumerate_cva6_r4(c: &mut Criterion) {
    let spec = cva6_r4_spec();
    c.bench_function("enumerate/cva6_r4_2M", |b| {
        b.iter(|| {
            let iter = EnumerateIter::new(black_box(&spec));
            let count = iter.count();
            black_box(count);
        })
    });
}

fn bench_evaluate_cva6_full(c: &mut Criterion) {
    let spec = cva6_full_spec();
    let combos = expand_all(&spec).unwrap();
    c.bench_function("evaluate/cva6_full_33M", |b| {
        b.iter(|| {
            let results = evaluate_all(
                black_box(&spec),
                black_box(combos.clone()),
                &ConstraintRegistry::default(),
                &ProjectorRegistry::default(),
            );
            let count = results.iter().filter(|r| r.passed).count();
            black_box(count);
        })
    });
}

fn bench_evaluate_cva6_r4(c: &mut Criterion) {
    let spec = cva6_r4_spec();
    let combos = expand_all(&spec).unwrap();
    c.bench_function("evaluate/cva6_r4_2M", |b| {
        b.iter(|| {
            let results = evaluate_all(
                black_box(&spec),
                black_box(combos.clone()),
                &ConstraintRegistry::default(),
                &ProjectorRegistry::default(),
            );
            let count = results.iter().filter(|r| r.passed).count();
            black_box(count);
        })
    });
}

fn bench_structural_enum_cva6_full(c: &mut Criterion) {
    let spec = cva6_full_spec();
    c.bench_function("struct_enum/cva6_full_33M", |b| {
        b.iter(|| {
            let count = StructuralEnum::new(black_box(&spec)).count();
            black_box(count);
        })
    });
}

fn bench_structural_enum_cva6_r4(c: &mut Criterion) {
    let spec = cva6_r4_spec();
    c.bench_function("struct_enum/cva6_r4_2M", |b| {
        b.iter(|| {
            let count = StructuralEnum::new(black_box(&spec)).count();
            black_box(count);
        })
    });
}

fn bench_validate_cva6_r4(c: &mut Criterion) {
    let spec = cva6_r4_spec();
    c.bench_function("validate/cva6_r4_2M", |b| {
        b.iter(|| {
            let space = validate_into_space(black_box(&spec), &ConstraintRegistry::default());
            let count = space.iter().count();
            black_box(count);
        })
    });
}

fn bench_enumerate_all_cva6_full(c: &mut Criterion) {
    let spec = cva6_full_spec();
    let all_checks = ConstraintRegistry::default().build_all(&spec.constraints, &spec.fields);
    let evaluator = ProjectorRegistry::default()
        .resolve(&spec.projector, &spec.fields)
        .unwrap();
    c.bench_function("enumerate_all/cva6_full_33M", |b| {
        b.iter(|| {
            let mut passed = 0usize;
            let mut total = 0usize;
            for combo in EnumerateIter::new(black_box(&spec)) {
                let mut ok = true;
                for check in &all_checks {
                    if !check.allows(combo.point.coordinates()) {
                        ok = false;
                        break;
                    }
                }
                let _proj = evaluator.evaluate(&combo.point);
                if ok {
                    passed += 1;
                }
                total += 1;
            }
            black_box((total, passed));
        })
    });
}

fn bench_structural_verify_cva6_full(c: &mut Criterion) {
    let spec = cva6_full_spec();
    let all_checks = ConstraintRegistry::default().build_all(&spec.constraints, &spec.fields);
    let evaluator = ProjectorRegistry::default()
        .resolve(&spec.projector, &spec.fields)
        .unwrap();
    c.bench_function("structural_verify/cva6_full_33M", |b| {
        b.iter(|| {
            let mut passed = 0usize;
            let mut total = 0usize;
            for combo in StructuralEnum::new(black_box(&spec)) {
                let mut ok = true;
                for check in &all_checks {
                    if !check.allows(combo.point.coordinates()) {
                        ok = false;
                        break;
                    }
                }
                let _proj = evaluator.evaluate(&combo.point);
                if ok {
                    passed += 1;
                }
                total += 1;
            }
            black_box((total, passed));
        })
    });
}

fn bench_constraint_check(c: &mut Criterion) {
    let spec = ibex_spec();
    let checks = ConstraintRegistry::default().build_all(&spec.constraints, &spec.fields);
    let combo = expand_all(&spec)
        .unwrap()
        .into_iter()
        .find(|c| c.values[0] == 4 && c.values[1] == 4)
        .unwrap();
    c.bench_function("constraint/check_single", |b| {
        b.iter(|| {
            let mut passes = true;
            for check in &checks {
                if !check.allows(black_box(combo.point.coordinates())) {
                    passes = false;
                    break;
                }
            }
            black_box(passes);
        })
    });
}

// ===========================================================================
// Criterion configuration
// ===========================================================================

// Fast microbenchmarks (ns-µs scale): high sample count for tight CIs.
criterion_group!(
    name = coordspace;
    config = Criterion::default().sample_size(1000);
    targets = bench_space_lookup_valid, bench_space_lookup_invalid,
              bench_constraint_check
);

// Expand/enumerate: ms scale for ibex/cva6, µs for small.
criterion_group!(
    name = expand;
    config = Criterion::default().sample_size(100);
    targets = bench_expand_small, bench_enumerate_small,
              bench_expand_medium, bench_enumerate_medium,
              bench_expand_ibex, bench_enumerate_ibex,
              bench_expand_cva6_r4, bench_enumerate_cva6_r4
);

// Light evaluation: small/medium scale, runs in µs-ms.
criterion_group!(
    name = eval_light;
    config = Criterion::default().sample_size(100);
    targets = bench_evaluate_small, bench_structural_enum_small, bench_validate_small,
              bench_evaluate_medium, bench_structural_enum_medium, bench_validate_medium
);

// Heavy evaluation: ibex/cva6 scale, runs in seconds per iter.
// 10 samples is practical: ibex evaluate = 36s, cva6 evaluate = 14s.
criterion_group!(
    name = eval_heavy;
    config = Criterion::default().sample_size(10);
    targets = bench_evaluate_ibex, bench_structural_enum_ibex, bench_validate_ibex,
              bench_evaluate_cva6_full, bench_evaluate_cva6_r4, bench_structural_enum_cva6_r4, bench_validate_cva6_r4,
              bench_structural_enum_cva6_full, bench_structural_verify_cva6_full
);

criterion_group!(
    name = eval_cva6_full;
    config = Criterion::default().sample_size(3);
    targets = bench_evaluate_cva6_full, bench_enumerate_all_cva6_full
);

criterion_main!(coordspace, expand, eval_light, eval_heavy, eval_cva6_full);
