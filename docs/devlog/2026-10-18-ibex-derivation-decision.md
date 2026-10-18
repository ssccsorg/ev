# The Ibex fixture stays a transcription with a pinned source

Issue #62 asked whether ev should derive the Ibex `(funct7, funct3)` mapping
from `ibex_decoder.sv` the way `tests/cva6_derivation.rs` derives the CVA6
fixtures from the decoder mask table. The issue named the alternative
explicitly: if the reader is not wanted, restate what the fixtures' claim
actually rests on.

## Decision

ev does not grow an RTL reader for the Ibex decoder. The Ibex fixtures remain a
hand transcription with a pinned source.

## Why

The CVA6 gate works because the decoder carries a literal `CoproInstr[]` table.
Reading it is a small, mechanical extraction. The Ibex decoder states legality
as behavioural `case` / `casez` code with parameter-gated branches, so deriving
the accepted pairs means evaluating a SystemVerilog subset under a fixed
parameter binding. That is an RTL reader, a capability of its own, and it moves
ev from checking a spec to elaborating a design. The project's scope statement
keeps ev as the atomic verifier: the engine is the vessel, breadth of targets
is not its to grow, and the provenance machinery is what ev owns.

The source pin already does what the scope requires. It makes a decoder change
detectable through the file digest and the commit, and it makes a transcription
edit visible through the fixture digests. The count assertions pin the fixture's
own behaviour. What remains unchecked is the transcription itself, and the
fixture now says so.

## What changed

- The two Ibex fixture headers state that the mapping is a hand transcription,
  that the pin checks the decoder and the fixture digests, and that it does not
  confirm the pairs.
- `tests/ibex_source_pin.rs` says the same in its module and test docs.
- `tests/fixtures/ibex/decoder_pin.json` restates the fixture digests for the
  comment-only edit. The decoder path, commit, and source digest are unchanged.
- The README's fixture provenance and validation tables, and the `AGENTS.md`
  source-pin section, state the decision.
- No verification code, constraint, projector, or count changed.

## Consequence for a consumer

An Ibex fixture is an imported claim with a pinned source. A consumer that needs
a mechanically derived Ibex mapping must derive it outside ev, or accept the
transcription as the claim. The CVA6 fixtures remain the pair whose accepted
set is re-derived by a gate.

## Verification

```bash
cargo test --release                                       # 156 passed, 0 ignored
cargo clippy --all-targets -- -D warnings                  # clean
cargo clippy --all-targets --no-default-features -- -D warnings  # clean
bash run.sh --verify                                       # exit 0
```

---

## References

- ev issue #62, issue #58 (the pin this follows), issue #56 (the CVA6 gate)
- `tests/fixtures/ibex/rv32imcb.xif.yaml`,
  `tests/fixtures/ibex/rv32imcb_imm.xif.yaml`,
  `tests/fixtures/ibex/decoder_pin.json`
- `tests/ibex_source_pin.rs`
