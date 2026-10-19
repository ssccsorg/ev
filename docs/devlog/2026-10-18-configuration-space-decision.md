# Compile-time configuration is not an encoding field

Issue #36 asked whether ev should model Ibex's compile-time parameter space.
The artifact the issue targeted, `tests/fixtures/ibex/alu_ext.xif.yaml`, is
gone, and the real Ibex encoding fixtures are `rv32imcb.xif.yaml` and
`rv32imcb_imm.xif.yaml`. What remained was the question of scope.

## Decision

ev classifies encoding spaces, not compile-time configuration spaces. A
configuration selects which instructions exist; it is not a field of an
instruction. Each spec models one configuration, and the fixture states the
parameter binding it assumes.

## Why

`RV32B`, `RV32M`, `RV32ZC`, and `RegFile` are elaboration parameters. A
parameter combination changes the set of legal opcodes, while each opcode that
exists has a fixed encoding. The current constraint language relates values
inside one encoding (`oneof`, `cross`, `enable_mask`, and the rest), so it
cannot express "the Zbt ternary operations are present when `RV32B` is not
None" without adding a spec-level configuration axis.

That axis would select which spec is being verified, not which encoding is
legal. It is a product change of the same order as the decode projector, and
it needs its own design note and milestone. Folding it into the encoding
language would make every constraint's meaning depend on a second quantifier,
which is a larger semantic change than the fixtures justify.

## What the fixture states

The Ibex fixtures already name the configuration in their headers: RV32BFull
and RV32MFast with `LV_A == LV_B == 0` for `rv32imcb`, and RV32BFull for
`rv32imcb_imm`. The source pin records the same binding. A consumer that needs
another configuration writes or imports a spec for it.

`tests/fixtures/common/enable_mask_demo.xif.yaml` remains a synthetic coverage
fixture, not an Ibex configuration model. Its header says so.

## Verification

No code changed. The full suite, clippy in both feature states, and
`run.sh --verify` are the check.

---

## References

- ev issue #36, issue #51 (the fixture relocation), issue #7 (core targeting)
- `tests/fixtures/ibex/rv32imcb.xif.yaml`,
  `tests/fixtures/ibex/rv32imcb_imm.xif.yaml`,
  `tests/fixtures/ibex/decoder_pin.json`
- `tests/fixtures/common/enable_mask_demo.xif.yaml`
