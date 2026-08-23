//! ev benchmarks — exhaustive verification pipeline performance.
//!
//! This file is the performance reference for the ExaVerif (ev) verification
//! pipeline. It measures the two enumeration strategies head to head under
//! identical conditions:
//!
//! - `evaluate_all` (standard): materializes the full Cartesian product of
//!   field domains (O(N)) and runs every constraint check on every
//!   combination.
//! - `struct_enum` (structural, synTagma): encodes the constraints into the
//!   enumeration space and visits only the valid combinations (O(V)),
//!   lazily, with O(1) memory.
//!
//! Methodology
//! -----------
//! Both pipelines are Rust, compiled in the same release profile, and
//! measured by the same criterion 0.5 harness. The speedup is therefore the
//! enumeration-strategy difference alone (the algorithmic O(N) vs O(V)
//! gain), not a language difference. Absolute times are machine-specific;
//! the ratio is not.
//!
//! Fixture reference
//! -----------------
//! | Fixture    | Source fixture                      | Raw combos | Valid combos | Density | evaluate | struct_enum | Speedup |
//! |------------|-------------------------------------|-----------:|-------------:|--------:|---------:|------------:|--------:|
//! | Small      | synthetic, 3 fields x 3            | 27         | 27           | 100%    | 4.87 µs  | 2.86 µs      | 1.7x    |
//! | Medium     | synthetic, ibex-like cross         | 32,768     | 20,480       | 62.5%   | 5.99 ms  | 3.02 ms      | 2.0x    |
//! | Tagma      | tests/fixtures/tagma/tagma_decoder | 65,536     | 11,172       | 17.0%   | 9.74 ms  | 5.94 ms      | 1.6x    |
//! | Ibex R-type| tests/fixtures/ibex/rv32imcb...    | 524,288    | 92,160       | 17.6%   | 297 ms   | 9.89 ms      | 30x     |
//! | CVA6 R4    | tests/fixtures/cva6/xif_ref_r4...   | 16,384     | 2,560        | 15.6%   | 4.13 ms  | 274 µs       | 15x     |
//! | CVA6 full  | tests/fixtures/cva6/xif_ref...      | 33,554,432 | 196,608      | 0.6%    | 13.1 s   | 18.8 ms      | ~700x   |
//!
//! The Tagma decoder's ge/le constraints are runtime-only, so its structural
//! density is 100% and the 1.6x gain is the enumeration overhead difference,
//! not a sparse-space effect; its table density (17%) is the valid/raw
//! ratio, not the structural filter ratio.
//!
//! The CLI verify path (`evaluate_structural`: raw total + structural
//! subset + evaluate_all on the valid rows) measures 36.3 ms on the CVA6
//! full fixture; the ~0.2 s end-to-end CLI number includes YAML parse and
//! process startup.
//!
//! The CVA6 fixtures are derived from the CVA6 hardware decoder mask table
//! (cvxif_instr_pkg.sv, instr_decoder.sv) at commit 6544a714c; see the
//! syntagma verification devlog for the decoder analysis.
//!
//! The speedup follows approximately `k / D`, where D = V/N is the valid
//! density and k is a fixture-dependent constant (about 1.3-5.5 in the
//! measured set). Sparse spaces gain the most; dense spaces gain little.
//! The Tagma row is the exception: its structural density is 100%, so the
//! model does not apply to it.
//!
//! Correctness guarantee
//! ---------------------
//! `struct_enum_validity/cva6_full_33M` asserts on every run that the number
//! of emitted combinations equals the number that satisfy the full
//! constraint set. A structural enumeration regression fails the benchmark;
//! the same invariant is guarded by tests/structural_enum.rs in CI.
//!
//! Running
//! -------
//! ```text
//! cargo bench                 # all groups (heavy; the full-space evaluate
//!                             #   benchmark takes ~13 s per sample and
//!                             #   peaks at several GB (materialized Vec)
//! cargo bench -- cva6_full    # full-space CVA6 group only
//! cargo bench -- expand       # expand/enumerate group only
//! cargo bench -- "struct_enum/cva6"   # structural enumeration only
//! cargo bench -- struct_enum_validity # correctness guard (must stay green)
//! ```

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::collections::BTreeMap;

use ev::spec::{ConstraintSpec, FieldSpec, ProjectorSpec, VerificationSpec};
use ev::verify::compose::{coords_to_coord_vec, expand_all, EnumerateIter, StructuralEnum};
use ev::verify::evaluate::{evaluate_all, evaluate_structural, validate_into_space};
use ev::verify::registry::{Check, ConstraintRegistry, ProjectorRegistry};

// ===========================================================================
// Fixtures
// ===========================================================================
//
// Each fixture is a VerificationSpec: a set of field domains plus a
// constraint set. The raw space is the product of the field domains; the
// valid space is the subset that satisfies every constraint. The
// combination counts below are the measured reference values (release
// build, single Apple silicon core).

/// Small spec: 3 fields x 3 values = 27 raw combos, 27 valid (100% density).
///
/// No constraints, so `expand_all` and `StructuralEnum` visit the same 27
/// points; the benchmark measures the baseline per-element cost of each
/// pipeline (the 1.7x ratio is the constant overhead floor).
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

/// Medium spec: 5 fields x 8 values = 32,768 raw combos, 20,480 valid
/// (62.5% density).
///
/// Synthetic ibex-like space: a `cross` constraint restricts funct3 per
/// funct7 (funct7=0 allows all 8 funct3; funct7=4 allows {1,4,5,6,7};
/// funct7=5 allows {1,2,3,4,5,6,7}; other funct7 values are unrestricted).
/// Dense enough that the structural gain is modest (2.1x).
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

/// Ibex rv32imcb spec: 524,288 raw combos, 92,160 valid (17.6% density).
///
/// Real fixture `tests/fixtures/ibex/rv32imcb.xif.yaml`, derived from the
/// Ibex OPCODE_OP decoder (ibex_decoder.sv) under RV32BFull. A single
/// `cross` constraint (funct7 -> funct3) models the main case plus the Zbt
/// ternary override; there are no runtime-only constraints, so the
/// structural path carries zero per-combination checks.
fn ibex_spec() -> VerificationSpec {
    let path = std::path::Path::new("tests/fixtures/ibex/rv32imcb.xif.yaml");
    VerificationSpec::from_yaml(path).expect("failed to load rv32imcb fixture")
}

/// CVA6 XIF ref R4 spec: 16,384 raw combos, 2,560 valid (15.6% density).
///
/// Real fixture `tests/fixtures/cva6/xif_ref_r4.xif.yaml`, derived from the
/// CVA6 hardware decoder mask table at commit 6544a714c. R4 format: func2 =
/// bits[26:25], rs3 fixed to 0, sampled registers (rs1, rd in 0..3, rs2 in
/// 0..31). Accepted custom-3 encodings: funct3=0/func2=00 (NOP) and
/// funct3=1/func2 in 0..3 (ADD family).
fn cva6_r4_spec() -> VerificationSpec {
    let path = std::path::Path::new("tests/fixtures/cva6/xif_ref_r4.xif.yaml");
    VerificationSpec::from_yaml(path).expect("failed to load cva6 xif ref r4 fixture")
}

/// CVA6 XIF ref full spec: 33,554,432 raw combos, 196,608 valid (0.6%
/// density).
///
/// Real fixture `tests/fixtures/cva6/xif_ref.xif.yaml`, derived from the
/// CVA6 hardware decoder mask table at commit 6544a714c. Flat R-type with
/// the full register range (rs1, rs2, rd in 0..31). Accepted custom-3
/// encodings: funct3=0/funct7=0 (NOP) and funct3=1/funct7 in 0..4 (ADD,
/// DOUBLE_RS1, DOUBLE_RS2, ADD_MULTI, ADD_RS3_R). The 0.6% density makes
/// this the headline sparse case: ~700x structural speedup.
fn cva6_full_spec() -> VerificationSpec {
    let path = std::path::Path::new("tests/fixtures/cva6/xif_ref.xif.yaml");
    VerificationSpec::from_yaml(path).expect("failed to load cva6 xif ref fixture")
}

/// Tagma decoder spec: 65,536 raw combos, 11,172 valid (17.0% density).
///
/// Real fixture `tests/fixtures/tagma/tagma_decoder.xif.yaml`, the input
/// domain contract of the syntagma 3-axis Hangul decoder. Two runtime
/// constraints (ge 0xAC00, le 0xD7A3) plus the tagma_decode projector that
/// packs the axis decomposition into the golden-anchor layout.
fn tagma_spec() -> VerificationSpec {
    let path = std::path::Path::new("tests/fixtures/tagma/tagma_decoder.xif.yaml");
    VerificationSpec::from_yaml(path).expect("failed to load tagma decoder fixture")
}

// ===========================================================================
// Domain expansion: expand_all (Vec) vs EnumerateIter
// ===========================================================================
//
// `expand_all` materializes the full Cartesian product as a Vec
// (O(N) memory and time). `EnumerateIter` streams the same raw product
// lazily (O(1) memory) without constraint filtering; the comparison isolates
// the cost of the Vec materialization itself. Neither applies constraints,
// so the counts equal the raw space.

/// expand_all on the small spec (27 combos): Vec materialization cost.
fn bench_expand_small(c: &mut Criterion) {
    let spec = small_spec();
    c.bench_function("expand/small_27", |b| {
        b.iter(|| {
            let combos = expand_all(black_box(&spec)).unwrap();
            black_box(combos);
        })
    });
}

/// EnumerateIter on the small spec (27 combos): lazy streaming cost.
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

/// expand_all on the medium spec (32,768 combos).
fn bench_expand_medium(c: &mut Criterion) {
    let spec = medium_spec();
    c.bench_function("expand/medium_32k", |b| {
        b.iter(|| {
            let combos = expand_all(black_box(&spec)).unwrap();
            black_box(combos);
        })
    });
}

/// EnumerateIter on the medium spec (32,768 combos).
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

/// expand_all on the Ibex fixture (524,288 combos, ~80 MB Vec).
fn bench_expand_ibex(c: &mut Criterion) {
    let spec = ibex_spec();
    c.bench_function("expand/ibex_524k", |b| {
        b.iter(|| {
            let combos = expand_all(black_box(&spec)).unwrap();
            black_box(combos);
        })
    });
}

/// EnumerateIter on the Ibex fixture (524,288 combos).
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
// Evaluation: evaluate_all (Vec) vs struct_enum vs validate_into_space
// ===========================================================================
//
// `evaluate_all` is the standard pipeline: expand + per-combination
// constraint checks (O(N)). `struct_enum` is the Tagma-based structural
// pipeline: it emits only the valid combinations (O(V)). `validate_into_space`
// places every structurally valid combination into a DynCoordSpace,
// deduplicating identical coordinate paths. The trio is benchmarked on each
// fixture to expose the O(N) vs O(V) gap.

/// evaluate_all on the small spec: 27 combos through the full check set.
fn bench_evaluate_small(c: &mut Criterion) {
    let spec = small_spec();
    c.bench_function("evaluate/small_27", |b| {
        b.iter(|| {
            let combos = expand_all(black_box(&spec)).unwrap();
            let results = evaluate_all(
                black_box(&spec),
                combos,
                &ConstraintRegistry::default(),
                &ProjectorRegistry::default(),
            );
            black_box(results);
        })
    });
}

/// struct_enum on the small spec: 27 valid combos, no checks.
fn bench_structural_enum_small(c: &mut Criterion) {
    let spec = small_spec();
    c.bench_function("struct_enum/small_27", |b| {
        b.iter(|| {
            let count = StructuralEnum::new(black_box(&spec)).count();
            black_box(count);
        })
    });
}

/// validate_into_space on the small spec: CoordSpace placement cost.
fn bench_validate_small(c: &mut Criterion) {
    let spec = small_spec();
    c.bench_function("validate/small_27", |b| {
        b.iter(|| {
            let space = validate_into_space(black_box(&spec), &ConstraintRegistry::default());
            black_box(space);
        })
    });
}

/// evaluate_all on the medium spec: 32,768 combos through the check set.
fn bench_evaluate_medium(c: &mut Criterion) {
    let spec = medium_spec();
    c.bench_function("evaluate/medium_32k", |b| {
        b.iter(|| {
            let combos = expand_all(black_box(&spec)).unwrap();
            let results = evaluate_all(
                black_box(&spec),
                combos,
                &ConstraintRegistry::default(),
                &ProjectorRegistry::default(),
            );
            black_box(results);
        })
    });
}

/// struct_enum on the medium spec: 20,480 valid combos (2.1x).
fn bench_structural_enum_medium(c: &mut Criterion) {
    let spec = medium_spec();
    c.bench_function("struct_enum/medium_32k", |b| {
        b.iter(|| {
            let count = StructuralEnum::new(black_box(&spec)).count();
            black_box(count);
        })
    });
}

/// validate_into_space on the medium spec.
fn bench_validate_medium(c: &mut Criterion) {
    let spec = medium_spec();
    c.bench_function("validate/medium_32k", |b| {
        b.iter(|| {
            let space = validate_into_space(black_box(&spec), &ConstraintRegistry::default());
            black_box(space);
        })
    });
}

/// evaluate_all on the Ibex fixture: 524,288 combos, ~282 ms.
fn bench_evaluate_ibex(c: &mut Criterion) {
    let spec = ibex_spec();
    c.bench_function("evaluate/ibex_524k", |b| {
        b.iter(|| {
            let combos = expand_all(black_box(&spec)).unwrap();
            let results = evaluate_all(
                black_box(&spec),
                combos,
                &ConstraintRegistry::default(),
                &ProjectorRegistry::default(),
            );
            let count = results.iter().filter(|r| r.passed).count();
            black_box(count);
        })
    });
}

/// struct_enum on the Ibex fixture: 92,160 valid combos (30x).
fn bench_structural_enum_ibex(c: &mut Criterion) {
    let spec = ibex_spec();
    c.bench_function("struct_enum/ibex_524k", |b| {
        b.iter(|| {
            let count = StructuralEnum::new(black_box(&spec)).count();
            black_box(count);
        })
    });
}

/// validate_into_space on the Ibex fixture.
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
//
// Microbenchmarks of the two validity-query mechanisms after the space is
// built: a CoordSpace path lookup (structural) versus a Vec scan over all
// combinations (standard). These measure the per-query cost of "is this
// combination valid?".

/// CoordSpace path lookup of a known valid Ibex combination (funct7=4,
/// funct3=4, PACK): the structural validity query.
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

/// CoordSpace path lookup of a known invalid Ibex combination (funct7=4,
/// funct3=0): a vacant slot is the invalid-encoding detector.
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
// CVA6 R4 fixture (16,384 raw / 2,560 valid)
// ===========================================================================

/// expand_all on the CVA6 R4 fixture (16,384 combos).
fn bench_expand_cva6_r4(c: &mut Criterion) {
    let spec = cva6_r4_spec();
    c.bench_function("expand/cva6_r4_16K", |b| {
        b.iter(|| {
            let combos = expand_all(black_box(&spec)).unwrap();
            black_box(combos);
        })
    });
}

/// EnumerateIter on the CVA6 R4 fixture (16,384 combos).
fn bench_enumerate_cva6_r4(c: &mut Criterion) {
    let spec = cva6_r4_spec();
    c.bench_function("enumerate/cva6_r4_16K", |b| {
        b.iter(|| {
            let iter = EnumerateIter::new(black_box(&spec));
            let count = iter.count();
            black_box(count);
        })
    });
}

/// evaluate_all on the CVA6 R4 fixture: 16,384 combos, ~3.33 ms.
fn bench_evaluate_cva6_r4(c: &mut Criterion) {
    let spec = cva6_r4_spec();
    c.bench_function("evaluate/cva6_r4_16K", |b| {
        b.iter(|| {
            let combos = expand_all(black_box(&spec)).unwrap();
            let results = evaluate_all(
                black_box(&spec),
                combos,
                &ConstraintRegistry::default(),
                &ProjectorRegistry::default(),
            );
            let count = results.iter().filter(|r| r.passed).count();
            black_box(count);
        })
    });
}

/// struct_enum on the CVA6 R4 fixture: 2,560 valid combos, ~274 µs (15x).
fn bench_structural_enum_cva6_r4(c: &mut Criterion) {
    let spec = cva6_r4_spec();
    c.bench_function("struct_enum/cva6_r4_16K", |b| {
        b.iter(|| {
            let count = StructuralEnum::new(black_box(&spec)).count();
            black_box(count);
        })
    });
}

/// validate_into_space on the CVA6 R4 fixture.
fn bench_validate_cva6_r4(c: &mut Criterion) {
    let spec = cva6_r4_spec();
    c.bench_function("validate/cva6_r4_16K", |b| {
        b.iter(|| {
            let space = validate_into_space(black_box(&spec), &ConstraintRegistry::default());
            let count = space.iter().count();
            black_box(count);
        })
    });
}

// ===========================================================================
// CVA6 full fixture (33,554,432 raw / 196,608 valid)
// ===========================================================================
//
// The headline sparse case. evaluate_all runs in ~13 s and peaks at
// several GB because the combination Vec is materialized per iteration;
// struct_enum streams 196,608 valid combinations in ~18.8 ms. Run this
// group with
// `cargo bench -- cva6_full` and keep the validity guard green.

/// evaluate_all on the CVA6 full fixture: 33.5M combos, ~13 s, several GB peak.
fn bench_evaluate_cva6_full(c: &mut Criterion) {
    let spec = cva6_full_spec();
    c.bench_function("evaluate/cva6_full_33M", |b| {
        b.iter(|| {
            // Full standard pipeline per iteration: materialize the Vec and
            // evaluate. No clone artifact: expand + evaluate is what the CLI
            // does, and cloning 33.5M combinations (~5 GB) would dominate.
            let combos = expand_all(black_box(&spec)).unwrap();
            let results = evaluate_all(
                black_box(&spec),
                combos,
                &ConstraintRegistry::default(),
                &ProjectorRegistry::default(),
            );
            let count = results.iter().filter(|r| r.passed).count();
            black_box(count);
        })
    });
}

/// struct_enum on the CVA6 full fixture: 196,608 valid combos, ~18.8 ms
/// (~700x vs evaluate at ~13 s).
fn bench_structural_enum_cva6_full(c: &mut Criterion) {
    let spec = cva6_full_spec();
    c.bench_function("struct_enum/cva6_full_33M", |b| {
        b.iter(|| {
            let count = StructuralEnum::new(black_box(&spec)).count();
            black_box(count);
        })
    });
}

/// evaluate_structural on the CVA6 full fixture: the exact pipeline `ev
/// verify` runs today (raw total + structural subset + evaluate_all on the
/// 196,608 valid rows). Benched at 36.3 ms; the ~0.2 s end-to-end CLI
/// number includes YAML parse and process startup.
fn bench_evaluate_structural_cva6_full(c: &mut Criterion) {
    let spec = cva6_full_spec();
    c.bench_function("evaluate_structural/cva6_full_33M", |b| {
        b.iter(|| {
            let (total, results) = evaluate_structural(
                black_box(&spec),
                &ConstraintRegistry::default(),
                &ProjectorRegistry::default(),
            )
            .expect("structural evaluation");
            let passed = results.iter().filter(|r| r.passed).count();
            black_box((total, passed));
        })
    });
}

/// validate_into_space on the CVA6 full fixture.
fn bench_validate_cva6_full(c: &mut Criterion) {
    let spec = cva6_full_spec();
    c.bench_function("validate/cva6_full_33M", |b| {
        b.iter(|| {
            let space = validate_into_space(black_box(&spec), &ConstraintRegistry::default());
            let count = space.iter().count();
            black_box(count);
        })
    });
}

/// Correctness guard for the structural pipeline: every combination the
/// structural iterator emits must satisfy the full constraint set. This is
/// the invariant the report's "generates only structurally valid
/// combinations" claim rests on; it fails (asserts) if the emitted count
/// and the full-check valid count diverge. The same invariant is guarded in
/// CI by tests/structural_enum.rs.
fn bench_structural_enum_validity_cva6_full(c: &mut Criterion) {
    let spec = cva6_full_spec();
    let checks = ConstraintRegistry::default().build_all(&spec.constraints, &spec.fields);
    c.bench_function("struct_enum_validity/cva6_full_33M", |b| {
        b.iter(|| {
            let mut emitted = 0usize;
            let mut valid = 0usize;
            for combo in StructuralEnum::new(black_box(&spec)) {
                emitted += 1;
                if checks.iter().all(|c| c.allows(combo.point.coordinates())) {
                    valid += 1;
                }
            }
            assert_eq!(
                emitted, valid,
                "struct_enum emitted {emitted} combinations but only {valid} satisfy the full constraint set"
            );
            black_box(valid);
        })
    });
}

// ===========================================================================
// Tagma decoder fixture (65,536 raw / 11,172 valid, 17.0% density)
// ===========================================================================

/// struct_enum on the Tagma decoder fixture: the 11,172 valid syllables of
/// the 65,536 code point space, ~5.9 ms.
fn bench_structural_enum_tagma(c: &mut Criterion) {
    let spec = tagma_spec();
    c.bench_function("struct_enum/tagma_decoder_64K", |b| {
        b.iter(|| {
            let count = StructuralEnum::new(black_box(&spec)).count();
            black_box(count);
        })
    });
}

/// evaluate_all on the Tagma decoder fixture: the naive path materializes
/// all 65,536 combos and runs the ge/le checks plus the tagma_decode
/// projection on the passing ones, ~9.7 ms.
fn bench_evaluate_tagma(c: &mut Criterion) {
    let spec = tagma_spec();
    c.bench_function("evaluate/tagma_decoder_64K", |b| {
        b.iter(|| {
            let combos = expand_all(black_box(&spec)).unwrap();
            let results = evaluate_all(
                black_box(&spec),
                combos,
                &ConstraintRegistry::default(),
                &ProjectorRegistry::default(),
            );
            let count = results.iter().filter(|r| r.passed).count();
            black_box(count);
        })
    });
}

/// evaluate_structural on the Tagma decoder fixture: the CLI verify path
/// with runtime-only constraints, where the structural subset is the full
/// space and the gain is the raw total computed without enumeration, ~8.8 ms.
fn bench_evaluate_structural_tagma(c: &mut Criterion) {
    let spec = tagma_spec();
    c.bench_function("evaluate_structural/tagma_decoder_64K", |b| {
        b.iter(|| {
            let (total, results) = evaluate_structural(
                black_box(&spec),
                &ConstraintRegistry::default(),
                &ProjectorRegistry::default(),
            )
            .expect("structural evaluation");
            let passed = results.iter().filter(|r| r.passed).count();
            black_box((total, passed));
        })
    });
}

// ===========================================================================
// Constraint check: single combination
// ===========================================================================

/// Cost of running the full Ibex constraint check set on one combination.
/// This is the per-element constant of the standard pipeline; multiplied by
/// N it dominates the O(N) wall-clock time.
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
// coordspace: CoordSpace path lookups and a single constraint check.
criterion_group!(
    name = coordspace;
    config = Criterion::default().sample_size(1000);
    targets = bench_space_lookup_valid, bench_space_lookup_invalid,
              bench_constraint_check
);

// expand/enumerate: Vec materialization vs lazy streaming, ms scale for
// ibex/cva6, µs for small.
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
              bench_evaluate_medium, bench_structural_enum_medium, bench_validate_medium,
              bench_evaluate_tagma, bench_structural_enum_tagma, bench_evaluate_structural_tagma
);

// Heavy evaluation: ibex/cva6 scale, runs in ~0.3 s to ~13 s per iter.
// 10 samples is practical: ibex evaluate ~0.3 s, cva6_r4 evaluate ~4 ms (release).
criterion_group!(
    name = eval_heavy;
    config = Criterion::default().sample_size(10);
    targets = bench_evaluate_ibex, bench_structural_enum_ibex, bench_validate_ibex,
              bench_evaluate_cva6_r4, bench_structural_enum_cva6_r4, bench_validate_cva6_r4
);

// Full CVA6 space (33M): evaluate is seconds per iter with ~8-15 GB peak,
// struct/validate/validity are ms-scale. Filter with `cargo bench -- cva6_full`.
criterion_group!(
    name = cva6_full;
    config = Criterion::default().sample_size(10);
    targets = bench_evaluate_cva6_full, bench_structural_enum_cva6_full,
              bench_evaluate_structural_cva6_full, bench_validate_cva6_full,
              bench_structural_enum_validity_cva6_full
);

criterion_main!(coordspace, expand, eval_light, eval_heavy, cva6_full);
