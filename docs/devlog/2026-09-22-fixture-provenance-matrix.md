# Fixture provenance matrix and the positioning it settles

Issue #57 answers two questions a reader of this repository could not answer from
one place: what each fixture is, where it came from, and which check keeps it
honest, and what ev verifies. The second question has a single answer that the
documentation had drifted away from: ev verifies encoding spaces, and the cores
it names are samples.

## The matrix

The README's Fixture Provenance table now lists every spec under
`tests/fixtures/`, sixteen of them, with its source and revision, its derivation,
its raw and valid counts, and the gate or assertion that covers it. Fixtures
without a gate are listed stating what does cover them rather than being left
out, and the three malformed specs that only the CLI tests exercise are in the
table for the same reason.

The `Based on` column of the previous table was a summary; the new table
separates what a fixture came from from what it verifies, and it replaces the
old table rather than sitting beside it, so there is one fixture list in the
README.

Three artifacts under `tests/fixtures/` are inputs to channels rather than
specs, and have their own short table: the CVA6 mask-table extraction, the demo
RTL for the design-only synthesis path, and the captured Yosys report.

## Two gaps the matrix exposed

Writing the table forced every fixture's provenance to be stated, and two
fixtures could not state theirs.

| Gap | Fixture set | Issue |
|---|---|---|
| The source is cited with no revision | `ibex/rv32imcb.xif.yaml`, `ibex/rv32imcb_imm.xif.yaml` cite `ibex/rtl/ibex_decoder.sv` with nothing pinning it, so a decoder change or a transcription error is undetectable | #58 |
| Counts are documented but not asserted | `xif_encoding` (8,192 / 48), `csr_access` (49,152 / 49,152), `all_pass` (1,024 / 1,024), `sample` (96 / 12) appear in the README and `AGENTS.md` with no `_verify_check` | #59 |

Both are recorded in the README beside the fixtures they concern and in the
devlog, and both now have issues. Neither is a documentation defect: #56 showed
what an unasserted fixture can hide when `xif_madd` accepted 3,072 combinations
the decoder rejects and nothing failed.

Two fixture headers also cited a source without the revision the matrix states.
`cva6/xif_madd.xif.yaml` and `cva6/xif_encoding.xif.yaml` now name
`6544a714c`, matching `xif_ref` and `xif_ref_r4`, which is the revision the
readable checkout carries.

## The positioning

The README introduction previously described the project by its samples: "The
CVA6 fixtures are derived from the hardware decoder mask table ...". The corpus
page went further: "The engine is validated against two production RISC-V
cores, CVA6 (OpenHW Group) and Ibex (lowRISC)", which reads as a scope claim
about those cores.

Both now state the same thing. The README says that ev verifies instruction
encoding spaces and that a target is described by its spec alone, with nothing
in the engine naming a core, and that CVA6, Ibex, and the Tagma decoder are
samples that exercise the engine. The corpus page in
`ssccs/docs/projects/ev/index.qmd` says the engine is a general verifier of
RISC-V encoding spaces, that its only knowledge of a target is the spec it is
given, and that the named cores and designs are samples verified from their own
sources.

The claim is checkable rather than aspirational: `ConstraintRegistry` and
`ProjectorRegistry` hold no target names, the engine's constraint and projector
types are declared in `spec/mod.rs` without any fixture in mind, and
`tests/cva6_derivation.rs` re-derives the CVA6 fixtures from the decoder's own
table rather than from anything the engine asserts.

## The principle behind it

`AGENTS.md` now carries the rule as design decision 6: target-specific knowledge
lives in the fixture and the spec, never in the engine. A sample's name, its
constants, and its sub-decoding belong to the fixture that states them.

Two incidents created it. Issue #55 found a projector carrying a sample's name
and base inside the engine, and issue #56 found fixtures leaving free a field
that the decoder pins. The first is a name that should not have been in the
engine; the second is a fact about the target that only the target's source can
settle. Both are regressions under the rule, and the derivation gate is its
guard on the fixture side.

## Verification

Documentation and fixture-header comments only; no engine, test, or fixture
semantics changed.

```bash
cargo test --release   # 130 passed, 0 ignored
cargo fmt --all --check
cargo clippy --all-targets
bash run.sh --verify
```

`cargo test --release` runs the derivation gate, which re-reads the two edited
fixture headers and confirms the fixtures still match the committed mask table.

## References

- ev issue #57, issue #56 (the derivation gate), issue #55 (the projector
  generalization), issue #51 (the docs-of-record move), issue #58, issue #59
- `ssccs/docs/projects/ev/index.qmd`, published as https://docs.ssccs.org/projects/ev/
