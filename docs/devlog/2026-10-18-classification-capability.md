# Classification is a capability, and the record names its engine

Issue #67 made classification the last channel behind the capability pattern.
Before this change `Commands::Verify` called `evaluate_structural` directly, so
the engine was not selectable, and `Classification` carried no engine identity
or claim class. A record could not say which engine produced it or whether the
claim covered the whole space.

## The seam

`src/verify/classifier.rs`:

| Item | Content |
|---|---|
| `Classify` | `name()`, `classify(spec, constraints, projectors) -> Result<Classification, String>` |
| `StructuralClassifier` | the default, `ev/structural`, structural enumeration |
| `NaiveClassifier` | the reference, `ev/naive`, `expand_all` + `evaluate_all` |

`EV_CLASSIFIER=structural` is the default. `EV_CLASSIFIER=naive` selects the
reference. An unknown value fails instead of falling back, as the simulation
backend selection does.

## The record

`Classification` now carries:

- `engine: EngineIdentity { name, version }`
- `claim: ClaimClass { Exhaustive, Partial }`

Both built-in engines claim `Exhaustive`. `Partial` is the class an imported
engine must set when it samples or truncates, so a partial observation cannot
be read as a complete classification. The point coordinates stay independent
of the engine, so verdicts from two engines are comparable at the same point.

Every reporter emits the two fields: the text summary prints `engine:` and
`claim:`, the CSV header comments and the trace start event carry them, and the
JSON payload adds `engine` and `claim`. The simulation Fact payload does the
same. The mock and Spike backends name themselves `ev/mock` and `ev/spike`.

## The comparison

`tests/classifier.rs` runs both engines on `common/sample` and on
`cva6/xif_ref_r4`. The first has only a runtime constraint, so the two verdict
lists are identical. The second has a `cross` constraint, so the structural
engine omits the points it rejects; the test requires every emitted point to
agree and every omitted point to be a failing one in the naive record.

The comparison is meaningful because issue #65 fixed the spec address first:
the same spec now hashes identically across processes, and the layout is part
of the address.

## What did not change

The structural pipeline remains the default, the fixture counts are unchanged,
and `run.sh --verify` still passes. The naive engine is opt-in and is not used
by the gates.

## Verification

```bash
cargo test --release                                       # 150 passed, 0 ignored
cargo clippy --all-targets -- -D warnings                  # clean
cargo clippy --all-targets --no-default-features -- -D warnings  # clean
bash run.sh --verify                                       # exit 0
```

---

## References

- ev issue #67, issue #65 (the address the comparison rests on)
- `src/verify/classifier.rs`, `src/classification/mod.rs`,
  `src/report/reporter.rs`, `src/main.rs`
- `tests/classifier.rs`, `tests/cli_test.rs`
