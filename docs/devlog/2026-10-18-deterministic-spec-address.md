# The spec address covers the layout and is stable across runs

Issue #65 found that `hash_spec` omitted the layout, so two specs that mapped
the same fields to different bits shared a content address. The issue also
owned the address determinism: every `cross` constraint stored its mapping in
a `HashMap`, and `hash_spec` hashes the constraint's `Debug` form, whose
iteration follows the per-process `RandomState`.

## What changed

| Before | After |
|---|---|
| `spec.layout` not hashed | the layout, including `word_width`, is hashed |
| `ConstraintSpec::Cross.mapping: HashMap<i64, Vec<i64>>` | `BTreeMap<i64, Vec<i64>>` |
| internal cross maps and `first_valid` used `HashMap` | `BTreeMap` |
| no negative test for the layout | `tests/spec_hash.rs` covers layout, width, position, comments, cross order |
| no cross-process check | the test runs the binary twice and compares `spec_hash` |

The hash is still the `Debug` form of each component. That is stable for a
given build, and the type change makes the one nondeterministic component
deterministic. A canonical serialization can replace it later without
changing the coverage rule.

## The coverage rule

The doc comment on `hash_spec` now states it: the hash covers everything that
changes the classified space, namely the target, each field's domain, the word
layout, the projector, and the constraints. A new `VerificationSpec` field
belongs in the hash when it changes which points pass or how they project.
Presentation-only data does not belong.

Comment and header edits to the YAML do not move the address because they are
not parsed. Reordering the keys of a `cross` mapping does not move it because
the mapping is a `BTreeMap`. Reordering the constraint list does move it,
because the sequence is part of the spec.

## Re-addressing

Every record id for a fixture that declares a layout moves once, because the
address now covers the layout. That is the intended re-addressing, and no
classified count changes.

## Verification

```bash
cargo test --release                                       # 143 passed, 0 ignored
cargo clippy --all-targets -- -D warnings                  # clean
cargo clippy --all-targets --no-default-features -- -D warnings  # clean
bash run.sh --verify                                       # exit 0
```

`tests/spec_hash.rs` includes `the_address_is_stable_across_processes`, which
runs `ev verify --json` twice on `common/enable_mask_demo.xif.yaml` and
compares the `spec_hash` inside the Fact payload. The fixture has a `cross`
constraint, so the test fails on the previous `HashMap` representation.

---

## References

- ev issue #65
- `src/report/reporter.rs` (`hash_spec`), `src/spec/mod.rs` (`ConstraintSpec::Cross`)
- `src/verify/compose.rs`, `src/verify/registry.rs`
- `tests/spec_hash.rs`
