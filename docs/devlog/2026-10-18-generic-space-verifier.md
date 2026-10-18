# The engine verifies a declared space, and RISC-V is a profile

Issue #70 raised the abstraction one level. The engine already classified a
declared discrete space, but the types and the parser carried the instruction
word as a fixed concept: `EncodingLayout` with a mandatory 32-bit `insn_width`,
an implicit R-type layout for every spec that omitted one, and a Spike backend
compiled unconditionally. A downstream certifier that consumes ev for a space
that is not a RISC-V instruction would have inherited all three.

## What moved

| Before | After |
|---|---|
| `spec::EncodingLayout` | `spec::WordLayout` |
| `word_width` was `insn_width: u32`, defaulting to 32 | `word_width: Option<u32>`, defaulting to absent |
| `VerificationSpec.encoding` | `VerificationSpec.layout` |
| Layout derivation in `format/xif.rs` | `profile::LayoutRegistry` |
| No named profile in the engine | `profile/riscv.rs` registers `riscv-r`, `riscv-r4`, alias `r4` |
| Spike compiled always | Spike compiled with `feature = "riscv"` |

`Cargo.toml` adds `default = ["riscv"]`, so the shipped binary and the existing
gates keep the RISC-V profiles. A build with `--no-default-features` has no
RISC-V layout and no Spike backend. The parser rejects `profile: riscv-r` in
that build instead of falling back, and `EV_SIM_BACKEND=spike` fails rather
than silently using the mock.

The parser keys are `profile` and `layout`, mutually exclusive. The old
`encoding` and `encoding_layout` keys remain serde aliases, and `insn_width`
remains an alias on a layout, so an existing spec still parses.

## The default is now no word

A spec that declares neither key has `layout: None`. The previous implicit
R-type layout made every target that declared no layout inherit bit positions
it never stated. The fixtures whose layout is actually consumed now declare
it:

| Fixture | Declaration | Consumer |
|---|---|---|
| `cva6/xif_ref.xif.yaml` | `profile: riscv-r` | derivation gate |
| `cva6/xif_ref_r4.xif.yaml` | `profile: riscv-r4` | derivation gate |
| `cva6/xif_madd.xif.yaml` | `profile: riscv-r4` | derivation gate |
| `cva6/xif_encoding.xif.yaml` | `profile: riscv-r` | Spike word check |

The Ibex fixtures were left untouched because no gate consumes their layout,
and their files are covered by the decoder source pin.

## The hash covers the layout

`hash_spec` now includes `VerificationSpec.layout`. Two specs that declare the
same fields and constraints but map them to different bits are different
spaces, and the address had not said so. Every record id for a fixture that
declares a layout moves once, because the address now covers it; that is the
intended re-addressing, and no classified count changes. The `cross` mapping
remains a `HashMap`, so the determinism defect recorded by issue #65 is
unchanged and still belongs to that issue.

## What did not change

No constraint type, projector, enumeration strategy, reporter, or count
changed. The acceptance check is the existing fixture table, which passes
unchanged, including the 33,554,432-point CVA6 space and the 11,172-point
Tagma space.

## Verification

```bash
cargo test --release                                       # 138 passed, 0 ignored
cargo clippy --all-targets -- -D warnings                  # clean
cargo clippy --all-targets --no-default-features -- -D warnings  # clean
cargo build --release --no-default-features
./target/release/ev verify --target tests/fixtures/cva6/xif_ref.xif.yaml
    # Error: unknown profile `riscv-r`: this build has no layout profiles
bash run.sh --verify                                       # exit 0
```

`run.sh --verify` includes the Yosys channel, the CVA6 derivation gate, the
Ibex source pin, and four Spike simulations. The two synthetic simulations
now run without a layout and skip the word check; the two CVA6 simulations
keep it.

## Follow-ups

`tagma-core` remains a hard dependency for `Coord`, `CoordPath`, and
`DynCoordSpace` in `verify`. It is not RISC-V-specific, so this change leaves
it in place, but it is the one domain-named dependency left in the core and
the next boundary to decide if ev is to ship without it.

---

## References

- ev issue #70
- `src/profile/mod.rs`, `src/profile/riscv.rs`, `src/spec/mod.rs`,
  `src/format/xif.rs`, `src/synth/backends/spike.rs`
- `tests/profile_registry.rs`
