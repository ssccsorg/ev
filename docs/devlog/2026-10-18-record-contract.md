# The record separates the point identity from the observation and the run

Issues #66 and #64 settle the same boundary from two sides. #66 asks what ev's
output must carry for a point. #64 asks which values are content and which
belong to the run. Both are answered without touching the verification
semantics: the counts, the enumeration, the constraints, and the projectors
are unchanged.

## The four decisions for a point

| Decision | Choice | Consequence |
|---|---|---|
| Identity of a point | `point_id = SHA256(spec_hash \|\| values)` | the verdict and projection are excluded, so the same coordinate keeps its identity across engines and across a changed verdict |
| Which coordinate | the field-value vector in `field_order` | vectors are spec-relative; `spec_hash` distinguishes specs; a packed word is a derived projection, not the identity, because a layout is optional |
| Space description | the payload is a pointer to the spec | `spec_hash` addresses the spec and `field_order` fixes the vector order; domains, layout, and projector stay in the spec |
| The rejected set | `failed = total - passed`, not emitted as rows | the omitted points are re-derivable from the spec; emitting 33,357,824 rejected points for `cva6/xif_ref` is not practical |

The observation id (`id`) remains `SHA256(spec_hash || values || passed ||
projection)`. The two ids answer different questions: `point_id` names the
coordinate, `id` names the observation.

## What changed

- `EvaluationEntry` carries `point_id` next to `id`.
- `VerificationReport` no longer carries `timestamp`. The Fact envelope
  already does, and the payload is the content that two runs must share.
- `SvGenerator` names generated RTL `<target>_<spec_hash[..16]>.sv` instead of
  a random suffix. The generated RTL is a pure function of the spec, so a
  stable name is safe and the source path stops being run-scoped.
- `YosysBackend` no longer writes `dot_path` or `netlist_path` into `extra`.
  Both pointed into the temporary work directory, which is deleted when the
  run returns, so the paths were stale as well as run-scoped.

## What did not change

No constraint, projector, enumeration, classification, or count changed. The
`id` algorithm is unchanged; the payload loses one field and gains another, so
a consumer that read `payload.timestamp` must read the Fact envelope, and a
consumer that used `id` as the point identity must use `point_id`.

## Verification

```bash
cargo test --release                                       # 156 passed, 0 ignored
cargo clippy --all-targets -- -D warnings                  # clean
cargo clippy --all-targets --no-default-features -- -D warnings  # clean
bash run.sh --verify                                       # exit 0
```

`tests/point_identity.rs` checks stability across two runs, comment edits, and
the structural and naive engines. `tests/run_metadata.rs` checks that the
verification and synthesis payloads are byte-identical across two runs and
that the generated RTL path is a function of the spec.

---

## References

- ev issue #66, issue #64, issue #65 (the address the identity is built on)
- `src/report/reporter.rs` (`hash_point`, `EvaluationEntry`, `VerificationReport`)
- `src/synth/mod.rs` (`generate_sv`), `src/synth/backends/yosys.rs`
- `tests/point_identity.rs`, `tests/run_metadata.rs`
