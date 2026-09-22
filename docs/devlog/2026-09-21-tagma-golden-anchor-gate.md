# Tagma golden anchor gate: a second channel for the decode projector

Issue #52 closes milestone 3 of issue #46 by adding an independent check of
the `tagma_decode` projector. This note records why the previous check was
not sufficient, which channel replaces it, and what the gate does on failure.

## Why the previous check was not sufficient

`tests/tagma_fixture.rs` pins the projector two ways. Three literal anchor
values (`pairs[0]`, `pairs[587]`, `pairs[11_171]`) are written without the
projector's constants, and the full valid domain is compared against the
decomposition formula written out a second time in Rust:

```rust
let expected = (k << 15) | ((k / 588) << 10) | (((k % 588) / 28) << 5) | (k % 28);
```

The literals are an independent pin at three offsets. The full-domain
comparison is self-consistent: the projector and the expectation are the
same implementation of the same formula, so a wrong stride, a wrong pack
order, or a wrong base survives both sides of that assertion. The syntagma
golden anchors were cited in the fixture header as the contract, while
nothing read them.

## The channel that replaced it

`tagma-core` (`sw/rust/core` in syntagma) is the reference engine:
`Coord::to_axes()` returns the `(initial, medial, final)` triple of an
offset, with `Coord::BASE = 0xAC00` and `N_VALID = 11_172`. Syntagma exports
that engine to `hw/rtl/golden_anchors.hex` in the layout
`offset[28:15] i[14:10] m[9:5] f[4:0]`.

That file is a generated build artifact (`hw/.gitignore`, `make
golden-export`), so syntagma's committed source of truth is the exporter.
`ev` already depends on the same crate (rev `205e3a0`, used by
`src/verify/compose.rs` and `src/verify/evaluate.rs`), which makes the
reference engine the channel that is always available, in CI included.

`tests/golden_anchor.rs` therefore checks two channels:

- Every valid code point: the projection equals the packing of
  `tagma_core::Coord::to_axes`, with the axis bounds asserted so stride
  drift cannot pass as a value change.
- When `EV_TAGMA_ANCHORS` points at a generated anchor file: line k equals
  the projection of code `0xAC00 + k`, over all 11,172 entries.

`run.sh --verify` resolves the anchor file from `EV_TAGMA_ANCHORS` or from a
sibling `${SYNTAGMA_DIR}/hw/rtl/golden_anchors.hex`, and prints whether the
artifact channel ran or was unavailable. An unavailable artifact is
reported, not passed over.

## Measured result

| Channel | Result |
|---|---|
| Reference engine (`tagma_core::Coord::to_axes`) | 11,172 / 11,172 agree |
| Generated `golden_anchors.hex` (syntagma, 100,548 bytes) | 11,172 / 11,172 agree |

Sensitivity of the artifact channel, checked by feeding deliberate
corruption:

| Input | Result |
|---|---|
| First 200 lines only | FAILED: "must hold one entry per valid code point" |
| Line 2 changed `00008001` to `00008002` | FAILED: "line 2 disagrees with the projection of code 0xAC01" |

## Coverage boundary

The two channels do not have the same reach, and the difference is worth
stating rather than leaving to discovery:

- CI checks out `ev` alone, and syntagma's anchor file is generated and
  untracked (`hw/.gitignore`), so CI runs the reference-engine channel and
  the artifact channel runs locally when a generated file exists. `run.sh`
  prints which of the two ran.
- This test restates the anchor bit layout in `pack`, so a layout error on
  which the projector and this test agree is caught only by the artifact
  channel, which compares against the file syntagma's exporter produced.
- A bare `cargo test` does not show the skip: cargo captures the test's
  `eprintln!` output. `run.sh --verify` prints the line that states the
  artifact channel was unavailable.
- `run.sh` runs the test binary twice in the default pipeline, once through
  `cargo test --release` in `code_checks` and once through this gate.
  Accepted, because the gate is what makes the artifact channel's status
  visible on stdout.
- `EV_TAGMA_ANCHORS` set to a path that is not a readable file fails before
  the test runs, naming the variable and the value, so a typo is not
  reported as a projection disagreement.

## Running

```bash
cargo test --release --test golden_anchor
EV_TAGMA_ANCHORS=../syntagma/hw/rtl/golden_anchors.hex cargo test --release --test golden_anchor
./run.sh --verify
```

## References

- ev issue #52, umbrella issue #46, fixture slice issue #47
- syntagma `hw/rtl/golden_anchors.hex` at `6a2f512ffb01bda2056b595da2141b29d8f55266`, `hw/tools/check_golden_anchors.py`
- golden anchor pattern: `ssccs/poc/baremetal_riscv/sv`
