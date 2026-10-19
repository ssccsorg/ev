# The enumeration property, measured

Issue #68 asked for the structural enumeration's claim to be stated in terms
that the repository can check. The claim had two parts that were true and one
that was not. The scaling group in `benches/bench.rs` now measures both axes,
and `FieldSpec::len` makes the raw total what the documentation said it was.

## The measurements

Fixed V, growing raw N: `a,b` in 0..7 with `cross a -> [a]`, and `c` in
0..C-1 restricted to `[0]` by `oneof`. V stays 8 while N is 64C.

| Raw N | filters | raw_total | iterator | end to end |
|------:|--------:|----------:|---------:|-----------:|
| 64,000 | 3.7 µs | 37 ns | 1.64 µs | 5.6 µs |
| 640,000 | 38.6 µs | 37 ns | 1.70 µs | 40.0 µs |
| 6,400,000 | 272 µs | 37 ns | 1.93 µs | 319 µs |

Growing k at fixed V: k two-value fields with all but the first fixed by
`oneof`. V stays 2 while N is 2^k.

| k | raw_total | end to end |
|--:|----------:|-----------:|
| 4 | 65 ns | 0.97 µs |
| 8 | 213 ns | 1.91 µs |
| 16 | 486 ns | 3.69 µs |
| 32 | n/a (N > MAX_COMBINATIONS) | 8.15 µs |

Medians from a release run with a 1 s warm-up and a 2 s measurement window.
Absolute times are machine-specific; the shapes are the claim.

## What the numbers say

The iterator is index-free. At fixed V, a 100x increase in the raw product
moves the iterator from 1.64 to 1.93 µs. The per-point cost is O(k), and
growing k at fixed V is linear: 4 to 32 fields moves 0.97 to 8.15 µs.

`raw_total_combinations` was not O(k) before this change. It called
`FieldSpec::expand().len()` for each field, so it materialized every domain.
`FieldSpec::len` now computes the cardinality arithmetically, and the measured
cost is flat at 37 ns across the 100x raw range.

The structural filter setup is O(sum of domain sizes), not O(k). It dominates
the end-to-end time when one field carries a large domain: 3.7 µs to 272 µs as
that domain grows 100x. The end-to-end pipeline is therefore
`O(sum domains + V*k)`, not `O(V)`. For the committed fixtures the domains are
small (8 to 128), so the 33M CVA6 raw product does not mean a 33M setup; a spec
with one very large field does pay for it.

## The memory boundary and the streaming decision

`evaluate_structural` collects the structurally valid combinations into a
`Vec` and `evaluate_all` builds a `Vec<Verdict>`, so memory is O(V). The largest
committed valid set is 196,608 points (CVA6 full). A streaming path would need
a sink-based record API, because `Classification` and every reporter take a
slice. That is a record-contract change of its own, and no consumer has asked
for a valid set that does not fit memory. The decision is to defer streaming
until one does.

## Verification

```bash
cargo test --release                                       # 161 passed, 0 ignored
cargo clippy --all-targets -- -D warnings                  # clean
cargo clippy --all-targets --no-default-features -- -D warnings  # clean
cargo bench -- scaling                                     # the two axes
bash run.sh --verify                                       # exit 0
```

`tests/domain_size.rs` pins `FieldSpec::len` against `expand().len()` for
positive, negative, stepped, empty, values-based, and default domains.

---

## References

- ev issue #68
- `src/spec/mod.rs` (`FieldSpec::len`), `src/verify/compose.rs`
  (`raw_total_combinations`, `StructuralEnum`, `structural_filters`)
- `benches/bench.rs` (scaling group), `tests/domain_size.rs`
