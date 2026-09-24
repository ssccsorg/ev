# The classification is the engine's output type

Issue #69 gave the classification a boundary type. Before this change the
engine's output was `verify::evaluate::Evaluation`, an internal that embeds
`Combination`, `Coordinates`, and `Point`, and three modules plus five test
files imported it to read four fields. The engine's private representation was
therefore the de facto interface of the whole system, and it was not on the
curated surface `lib.rs` re-exports.

## What the type is

`src/classification/mod.rs`:

| Type | Content |
|---|---|
| `Verdict` | `values`, `passed`, `projection`, `reason` |
| `Classification` | `field_order`, `total`, `verdicts`, plus `counts()`, `passed()`, `failed()` |

`counts()` carries the `passed <= total` invariant assert, which moved off the
reporter. `total` is the raw cartesian size, so the counts stay authoritative
when the verdict list holds only the structurally valid subset, which is what
the structural pipeline emits.

## What stays inside the engine

`evaluate_structural` returns `Classification`; `evaluate_all` returns
`Vec<Verdict>` and destructures `Combination` internally, so `Combination`,
`Coordinates`, and `Point` no longer leave `verify`. No module outside `verify`
imports `verify::evaluate` or `verify::compose`.

`evaluate_all` returns rows rather than a `Classification` on purpose: the
tests build both pipelines and compare them per point, and only
`evaluate_structural` has the raw total from the domains. Constructing a
`Classification` in `evaluate_all` would have to guess the total from the row
count, which is the confusion #66 settles.

## The emitted output is unchanged

34 files captured before and after, on the committed fixtures with the mock
backends: `verify` in all four formats for seven fixtures, `simulate` in four,
and `synth` in text and JSON. After normalising the run timestamp, 33 of 34 are
byte-identical. Text and CSV matched byte for byte with no normalisation.

## What the one difference was

`verify_common_enable_mask_demo.json` differed in `spec_hash` and in every
entry `id` derived from it. The refactor did not touch `hash_spec`; the fixture
had already been hashing nondeterministically. Confirmed by running the same
binary repeatedly, eight runs per fixture over all sixteen committed specs:

```
fixtures tested: 16
nondeterministic: 6
  common/enable_mask_demo        distinct=3/8
  cva6/xif_encoding              distinct=5/8
  cva6/xif_ref                   distinct=2/8
  cva6/xif_ref_r4                distinct=2/8
  ibex/rv32imcb                  distinct=8/8
  ibex/rv32imcb_imm              distinct=2/8
```

Six of the thirteen parseable fixtures produce a different `spec_hash` on
identical input, and they are exactly the six with a `cross` constraint, so the
correlation is complete in both directions. `ConstraintSpec::Cross` holds
`mapping: HashMap<i64, Vec<i64>>`, and `hash_spec` formats the constraint with
`{:?}`. `Debug` for a `HashMap` iterates in `RandomState` order, which is
seeded per process, so the address of the space moves between runs.

This is a defect against the stated claim that identical runs produce identical
records, and it is more than the omission issue #65 records: the address is not
merely incomplete, it is not a function of the spec. #65 now owns both, since
the fix is one pass over what the hash covers.

## Verification

```bash
cargo test --release          # 132 passed, 0 ignored
cargo fmt --all               # applied, tree clean
cargo clippy --all-targets    # clean under -D warnings
bash run.sh --verify          # exit 0
```

## References

- ev issue #69, issue #66 (what a point's record carries), issue #67 (the
  engine capability this is the precondition for), issue #65 (the address)
- `src/classification/mod.rs`, `src/verify/evaluate.rs`, `src/report/reporter.rs`
