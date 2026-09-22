# The Ibex decoder source is pinned

Issue #58 closes the last provenance gap the fixture matrix recorded. The two
RV32IMCB fixtures state that they model `ibex/rtl/ibex_decoder.sv` and name no
revision, so a decoder change between the transcription and today was
undetectable. The counts asserted in `run.sh` pin the fixtures' own behaviour,
not the decoder they claim to model.

This note records what the pin covers, what it deliberately does not, and why
the mapping's comparison with the decoder is a separate subject (issue #62).

## What the pin holds

`tests/fixtures/ibex/decoder_pin.json`:

| Field | Value |
|---|---|
| Source | `rtl/ibex_decoder.sv` |
| Revision | `f45407744572b2db9b04db1a7ba7cfb95a0fc811` on `master`, 2026-05-27 |
| Digest | `ba04566d91209d1f58d70a63405b137cfa28d76207d2e5bebdd5dcbf2d9fd88a` |
| Configuration | RV32BFull and RV32MFast, `LV_A == LV_B == 0` |
| Covered fixtures | `rv32imcb.xif.yaml`, `rv32imcb_imm.xif.yaml`, with their digests |

`tests/ibex_source_pin.rs` checks two things.

## The digest is the hard check, the revision is reported

A checkout at another commit whose `ibex_decoder.sv` has the pin's digest
describes the same decoding, which is what the fixtures were transcribed from,
so it passes with a note. A different digest fails and names the checkout
digest, the pinned digest, the revision, the branch, the date, and the
configuration. The issue asked for a revision mismatch to fail; the digest is
the check that implements the intent, since a commit that did not touch the
decoder cannot have changed the mapping.

## The fixture digests are in the pin

The pin also covers the fixture files it was taken against, and that check runs
wherever the tests run, CI included. It is a tripwire rather than a
verification: editing either fixture invalidates the pin, and the failure tells
the author to read the decoder again and restate the digest. Without it, a
fixture edit would silently inherit the pin's authority.

## What this does not do

No gate compares the fixtures' `(funct7, funct3)` mapping with the decoder. The
CVA6 gate can, because CVA6 states the accepted encodings as a literal
`CoproInstr[]` table. Ibex states them as behavioural code with parameter-gated
branches:

```systemverilog
5'b0_1101: illegal_insn = (RV32B != RV32BNone) ? 1'b0 : 1'b1;   // binvi
7'b000_0101: illegal_insn = (RV32B != RV32BNone) ? 1'b0 : 1'b1; // sext.h
```

Deriving the accepted pairs from that means evaluating an RTL subset under a
fixed parameter binding, which is a reader for one dialect and a capability of
its own. It is issue #62, and it is a decision rather than a chore.

Because of that gap, the fixture headers no longer say the mapping is
`verified against` the decoder. They say `transcribed from` the file at the
pinned revision, and state that no gate confirms the pairs. The earlier wording
implied a check that did not exist.

## Measured result

The pin channel with the checkout present:

```
tests/fixtures/ibex/rv32imcb.xif.yaml matches the pin
tests/fixtures/ibex/rv32imcb_imm.xif.yaml matches the pin
Ibex checkout ../ibex at f45407744572b2db9b04db1a7ba7cfb95a0fc811 matches the pin
```

Sensitivity, by mutating the pin and by pointing `IBEX_DIR` at a copy of the
decoder with one line appended:

| Change | Result |
|---|---|
| Source digest in the pin replaced with zeros | FAILED, naming the checkout digest, the pin's, the revision, the branch, the date, and the configuration |
| Pinned revision replaced with zeros, digest correct | passes, with the note that the file is unchanged |
| Fixture digest in the pin replaced with zeros | FAILED, telling the author to read the decoder again and restate the pin |
| `IBEX_DIR` at a copy of the decoder with a line appended | FAILED, naming both digests and the revision |

## Verification

```bash
cargo test --release          # 132 passed, 0 ignored
cargo fmt --all --check       # clean
cargo clippy --all-targets    # clean under -D warnings
bash run.sh --verify          # exit 0, "ibex source pin" reports the checkout and the match
```

## References

- ev issue #58, issue #62 (the derivation this defers), issue #57 (the matrix
  that found the gap), issue #56 (the CVA6 gate this mirrors in shape only)
- `ibex/rtl/ibex_decoder.sv` at `f4540774`, `tests/fixtures/ibex/decoder_pin.json`
