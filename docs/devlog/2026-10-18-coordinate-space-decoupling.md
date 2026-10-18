# The engine's coordinate space is its own type

`tagma-core` was the last domain-named runtime dependency in the library. It
supplied `Coord`, `CoordPath`, and `DynCoordSpace` to `verify`, and the
conversion through `Coord::new` restricted every field value to `u16`. A spec
with a negative value or a value above 65,535 was silently dropped from
`validate_into_space`, because the helper returned `None` for it. The engine's
space is the field-value vector, so the conversion was both a coupling and a
coverage bug.

## What changed

| Before | After |
|---|---|
| `tagma_core::Coord` / `CoordPath` | `verify::CoordSpace` over `Vec<i64>` |
| `validate_into_space -> DynCoordSpace<()>` | `validate_into_space -> CoordSpace` |
| `coords_to_coord_vec`, `coords_to_path` | removed |
| `tagma-core` in `[dependencies]` | `tagma-core` in `[dev-dependencies]` |

`CoordSpace` is a `BTreeSet<Vec<i64>>` with `place`, `contains`, `iter`,
`len`, and `is_empty`. It stores distinct coordinates and keeps values outside
the old `u16` range. `tests/coord_space.rs` covers idempotent placement,
containment, and the out-of-range case.

`tagma-core` remains in the repository only for `tests/golden_anchor.rs`, which
uses `Coord::to_axes` as the syntagma reference engine for the `decompose`
projection. That is a sample-specific cross-channel, so a dev-dependency is
the right boundary: a downstream consumer of the library does not build it.

## What did not change

No constraint, projector, enumeration, classification, or count changed. The
CLI pipelines and the fixture table are unchanged. The only behavior change is
in `validate_into_space`, which now includes coordinates the `u16` conversion
used to drop; it is a public helper used by tests and benchmarks, not by the
verification CLI.

## Verification

```bash
cargo test --release                                       # 156 passed, 0 ignored
cargo clippy --all-targets -- -D warnings                  # clean
cargo clippy --all-targets --no-default-features -- -D warnings  # clean
bash run.sh --verify                                       # exit 0
```

---

## References

- `src/verify/compose.rs` (`CoordSpace`), `src/verify/evaluate.rs`
  (`validate_into_space`), `Cargo.toml`
- `tests/coord_space.rs`, `tests/golden_anchor.rs`
