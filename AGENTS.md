# ev — ExaVerif: Agent Context Handoff

## Project Identity

Exhaustive verification CLI for RISC-V custom instruction extensions. Given
a YAML spec describing instruction fields and constraints, `ev` generates
every structurally valid combination, evaluates the constraints, and reports
pass/fail deterministically and exhaustively.

- Repository: `github.com/ssccsorg/ev`
- Language: Rust (edition 2021), Apache 2.0
- Version: 0.1.0 (pre-1.0, not published to crates.io)

## Scope

ev is the atomic verifier: given a spec it classifies the declared encoding
space exhaustively and deterministically, and it pins where the spec came from.
That is its whole responsibility.

The accumulation layer is not ev's. Making the records a queryable, superseding
corpus, comparing runs and engines, and the design-oracle product belong to
ExaSpec, which consumes ev's Facts; the FIH field alignment waits for nexus
(see Open Work). ev's obligation toward that layer is to emit a complete
classification that is reproducible and correctly addressed, which is what
issues #64, #65, and #68 are about, and to keep its engine one implementation
behind a capability interface (#67) so a more specialized verifier can be
routed in without changing what ev emits.

The engine is a vessel rather than the differentiator. The classification
behind the capability seam (#67) may be imported: an engine with different
performance, different accuracy, or coverage of another chip family can be
routed in without changing what ev emits, so breadth of targets is not ev's to
grow. The samples under `tests/fixtures/` exercise the vessel and its
channels, and what is ev's own is the machinery that makes an imported spec
trustworthy: a spec states its source, the derivation from that source is
checked where it can be mechanical, and the source is pinned where it cannot
(#56, #58, #62).

## Architecture

```text
src/
  main.rs           CLI (clap: verify, simulate, synth, fact decode)
  lib.rs            public re-exports
  classification/   Verdict, Classification — the engine's output type
  spec/             VerificationSpec, FieldSpec, ConstraintSpec, ProjectorSpec,
                    EncodingLayout, FieldBitMapping
  verify/
    compose.rs      domain expansion, raw_total_combinations, StructuralEnum
    evaluate.rs     evaluate_all, evaluate_structural, validate_into_space,
                    build_runtime_checks
    registry.rs     ConstraintRegistry, ProjectorRegistry, Check/Evaluator traits
  report/
    reporter.rs     ReporterCapable, Text/Json/Csv/Trace reporters
    fih.rs          Fact envelope (fact_type, origin, target, payload, timestamp,
                    parent_fact_id)
  format/
    xif.rs          YamlFormat, the XIF parser (FormatCapable)
  synth/
    mod.rs          GenerateRtl, RunSynthesis, SvGenerator, MockSynthesisBackend
    sim.rs          RunSimulation, MockSimBackend
    backends/       spike.rs (SpikeBackend), yosys.rs (YosysBackend)
benches/bench.rs    criterion reference, fixture table in the module doc
tests/              integration tests, fixtures under tests/fixtures/
docs/               private devlog only (see Documentation)
```

The README carries the same tree in user-facing form. Keep the two in step
when the layout changes.

## Current State

### Constraint and projector surface

- Constraint types (13): `range`, `even`, `eq`, `neq`, `lt`, `gt`, `le`,
  `ge`, `oneof`, `cross`, `bitmask`, `enable_mask`, `enable_set`.
- Structural constraints (`range`, `oneof`, `cross`, `bitmask`,
  `enable_mask`, `enable_set`) are encoded into the enumeration space, so
  invalid combinations are never generated. Runtime constraints (`eq`,
  `neq`, `lt`, `gt`, `le`, `ge`, `even`) are checked per emitted combination
  by `build_runtime_checks`.
- Projector types (4): `sum`, `identity`, `parity`, `decompose`. The last
  one splits one field into packed mixed-radix axes; the syntagma anchor
  layout `offset[28:15] i[14:10] m[9:5] f[4:0]` is the tagma fixture's
  instance of it, so no target name lives in the engine.

### CLI and pipelines

- `ev verify` runs `evaluate_structural`: the raw total from the field
  domains without enumeration, the structurally valid subset, then
  `evaluate_all` on that subset (issue #42).
  `ReporterCapable::report` takes the total and derives `failed = total -
  passed`, so counts stay authoritative when the evaluation list holds only
  the valid subset.
- `ev simulate` keeps `expand_all` and passes `evaluations.len()` as the
  total, so its behavior is unchanged.
- `ev synth --target <yaml>` generates RTL from a spec and synthesizes it;
  `ev synth --design <rtl> [--top <module>]` synthesizes an RTL file
  directly, with the top module defaulting to the file stem (issue #46,
  milestone 4). The two inputs are mutually exclusive, and `--top` applies
  to `--design` only.
- The naive path (`expand_all` + `evaluate_all`) remains for equivalence
  tests and benchmarks.

### Verification channels

- Structural equivalence: `tests/structural_enum.rs` pins
  `StructuralEnum` against `expand_all` per fixture, including the cross
  constraint wrap regression and the enable_mask parity invariant.
- Tagma decoder cross-channel: `tests/golden_anchor.rs` compares the
  `decompose` projections of the tagma fixture with the
  `tagma_core::Coord::to_axes` reference engine over all 11,172 offsets, and
  with a generated `hw/rtl/golden_anchors.hex` when `EV_TAGMA_ANCHORS` points
  at one.
  `run.sh --verify` reports the artifact channel as checked or unavailable.
- Spike backend: `src/synth/backends/spike.rs` generates a C program that
  re-implements the constraint model and the instruction-word assembly,
  cross-compiles it, and runs it under Spike plus pk. The C assembler is
  checked against Rust-computed reference words. This is not ISA-level
  execution: custom-3 opcodes are illegal in the base RISC-V ISA, so Spike
  never executes the custom words.
- CVA6 fixture derivation gate: `tests/cva6_derivation.rs` re-derives
  `cva6/xif_ref.xif.yaml`, `cva6/xif_ref_r4.xif.yaml`, and
  `cva6/xif_madd.xif.yaml` from `tests/fixtures/cva6/mask_table.json`, the
  committed extraction of the hardware decoder mask table (`cvxif_instr_pkg.sv`,
  `instr_decoder.sv`) at commit `6544a714c`. Per fixture it requires the
  table's accepted words and the fixture's accepted set to agree on the
  fixture's decision axes, and fails naming the entry when an entry masks a bit
  no declared field covers. With a checkout at hand (`CVA6_DIR`, default
  `../cva6`) it re-extracts the table and checks the commit and the file
  digests; `run.sh --verify` reports the source channel as checked or
  unavailable.
- Ibex source pin: `tests/ibex_source_pin.rs` compares a checkout with
  `tests/fixtures/ibex/decoder_pin.json`, which pins `rtl/ibex_decoder.sv` at
  `f4540774` (the digest, the assumed configuration, and the two `rv32imcb*`
  fixture files it covers). The digest is the hard check and the revision is
  reported, so a checkout at another commit describing the same decoder passes
  with a note. The fixture digests are checked wherever the tests run, so a
  fixture edit has to restate the pin. No gate compares the fixtures'
  `(funct7, funct3)` mapping with the decoder yet, which is why issue #62
  exists and why the fixture headers say `transcribed from`.
- Coverage gate: `scripts/coverage.sh` (cargo-llvm-cov, 80% lines and 80%
  regions) exercises the Spike, Yosys, and simulation backends with the
  instrumented binary.
- Synthesis channel: `YosysBackend` runs `read_verilog -sv`, `hierarchy`,
  `proc`, `synth`, then `tee -o <file> stat -json`. The report is read from
  `design` (aggregate) with `modules[\<top>]` as the fallback and the source
  of the per-cell-type breakdown. `--design` on
  `syntagma/hw/rtl/tagma_decoder.v` reports 478 cells with Yosys 0.65, the
  number the syntagma generic flow reports for the same RTL; a design input
  that yields no gate count fails `run.sh --verify`.

### Tests and fixtures

```bash
cargo test --release          # 132 tests: 83 lib, 28 CLI, 8 structural,
                              # 5 tagma, 2 golden anchor, 4 derivation, 2 source pin.
                              # None ignored.
cargo bench -- cva6_full      # full-space CVA6 group
cargo bench -- struct_enum_validity   # correctness guard, must stay green
./run.sh                      # fmt, clippy, build, test, verify
./run.sh --verify             # Yosys, fixtures, golden anchors, derivation gate, source pin, Spike
./run.sh --demo               # channel demo: the ssccs POC assembly golden anchors
./run.sh --coverage           # coverage gate
```

| Fixture | Raw combinations | Valid |
|---|---:|---:|
| `cva6/xif_ref.xif.yaml` | 33,554,432 | 196,608 |
| `cva6/xif_ref_r4.xif.yaml` | 16,384 | 2,560 |
| `cva6/xif_mac.xif.yaml` | 32,768 | 28,672 |
| `cva6/xif_madd.xif.yaml` | 32,768 | 1,024 |
| `cva6/xif_encoding.xif.yaml` | 8,192 | 48 |
| `ibex/rv32imcb.xif.yaml` | 524,288 | 92,160 |
| `ibex/rv32imcb_imm.xif.yaml` | 65,536 | 55,616 |
| `ibex/csr_access.xif.yaml` | 49,152 | 49,152 |
| `tagma/tagma_decoder.xif.yaml` | 65,536 | 11,172 |
| `tagma/tagma_demo_top.xif.yaml` | 11,172 | 11,172 |
| `common/enable_mask_demo.xif.yaml` | 524,288 | 4,096 |
| `common/all_pass.xif.yaml` | 1,024 | 1,024 |
| `common/sample.xif.yaml` | 96 | 12 |

`common/enable_mask_demo.xif.yaml` is a synthetic coverage fixture (oneof +
cross + enable_mask, identity projector). It is not an Ibex hardware model:
Ibex exposes standard extensions through compile-time parameters, and its
decoder fixtures are the `ibex/rv32imcb*.xif.yaml` pair (issue #36).

The table above is the count reference. The fixture inventory, with each
fixture's source, revision, derivation, and covering gate, is the README's
Fixture Provenance table; keep the counts in step between the two.

### Backends and environment

| Variable | Values | Effect |
|---|---|---|
| `EV_SIM_BACKEND` | `mock` (default), `spike` | Simulation backend |
| `EV_SYNTH_BACKEND` | `mock`, `yosys` (default) | Synthesis backend |
| `EV_SPIKE_BIN` | path | Spike binary |
| `EV_PK_PATH` | path | Proxy kernel |
| `EV_RISCV_CC` | command | RISC-V cross-compiler |
| `EV_TAGMA_ANCHORS` | path | Generated `golden_anchors.hex` for the tagma artifact channel |
| `SYNTAGMA_DIR` | path | Sibling syntagma checkout (default `../syntagma`), the artifact-channel fallback |
| `CVA6_DIR` | path | Sibling CVA6 checkout (default `../cva6`) for the derivation gate's source channel |
| `EV_UPDATE_MASK_TABLE` | `1` | Rewrite `tests/fixtures/cva6/mask_table.json` from a checkout at the pinned commit |
| `IBEX_DIR` | path | Sibling Ibex checkout (default `../ibex`) for the source pin's checkout channel |

## Key Design Decisions

1. Capability traits: `FormatCapable`, `ReporterCapable`, `GenerateRtl`,
   `RunSimulation`, `RunSynthesis`. New formats and backends require no
   pipeline change.
2. Constraints reference fields by name string, never by numeric axis, so
   YAML declaration order does not matter.
3. `ConstraintRegistry` and `ProjectorRegistry` use a builder pattern over
   boxed `Check` and `Evaluator` implementations, pluggable at runtime.
4. The library crate has no external tool coupling. The CLI resolves
   backends and tool paths from the environment.
5. The structural pipeline is the default and the naive pipeline is the
   reference it is tested against; keep both, and keep the count invariants
   pinned per fixture.
6. Target-specific knowledge lives in the fixture and the spec, never in the
   engine. The engine knows fields, bit positions, constraints, and
   projectors; a sample's name, its constants, and its sub-decoding belong to
   the fixture that states them. Two incidents created the rule: a projector
   carried a sample's name and base inside the engine (#55), and fixtures left
   a field free that the decoder pins (#56). A projector or engine path that
   names a core is a regression, and `tests/cva6_derivation.rs` is the guard
   on the fixture side.
7. ev makes no probabilistic claim. A run classifies every point of the
   declared space and nothing else, and identical runs produce identical
   records. Sampling, coverage percentages, and confidence bounds are outside
   the concept: a claim that rests on a sample belongs to a different tool, and
   comparing against such a tool measures that tool. The one percentage here is
   the code-coverage gate, which measures this repository's test suite and
   never a verification claim.

## How to Extend

- New constraint type: add a variant to `ConstraintSpec` in `spec/mod.rs`,
  a builder closure in `ConstraintRegistry::default()`, and an arm in
  `sv_constraint_assertion` in `synth/mod.rs`.
- New projector type: add a variant to `ProjectorSpec`, a builder in
  `ProjectorRegistry::default()`, and an arm in `sv_projector`.
- New input format: implement `FormatCapable`. New output format: implement
  `ReporterCapable` and pass the classification through.
- New classification engine: produce a `Classification` from a spec. The
  built-in engine is `verify::evaluate_structural`, reached in one call in
  `Commands::Verify`; #67 turns that call into a selectable capability with the
  naive pipeline (`expand_all` + `evaluate_all`) as the second implementation.

## Documentation

Public documentation of record lives in the ssccs corpus,
`ssccs/docs/projects/ev/`, published as https://docs.ssccs.org/projects/ev/.
`ev/docs` holds private development notes only; see `docs/README.md`.
Published numbers live in the corpus, not here.

## Dependency Note

`tagma-core` is a git dependency on `github.com/ssccsorg/tagma`, used by
`src/verify/compose.rs` (`Coord`, `CoordPath`) and `src/verify/evaluate.rs`
(`DynCoordSpace`). `Cargo.lock` pins the revision; `Cargo.toml` does not.
Syntagma plans to publish it to crates.io, at which point the dependency can
become a version.

## Git and Workflow

- Do not push; do not merge a pull request or any branch.
- New task subject: create a GitHub issue with labels, then branch as
  `{issue-number}-{subject-words-with-dashes}`.
- Commit format `{category}: {message}`, with `#{issue}` after the category
  on task branches.
- All artifacts, code, comments, and output are in English.

## Open Work

- Issue #62: derive the Ibex fixture mapping from the decoder's `case`
  statement. The decoder is behavioural code rather than a table, so this is a
  reader for an RTL subset and a capability decision, not fixture hygiene. The
  alternative, if the reader is not wanted, is to restate what the fixtures'
  claim actually rests on.
- Issue #44: execute the accepted CVA6 custom-3 encodings through the
  standard CVA6 tandem flow, which needs the external CVA6 repository. A
  sample's DV environment stays a sample concern: the encoding contract is
  checked by the derivation gate, without the external repository.
- Issue #18: `--interpret` for failure explanation, re-scoped to an
  OpenAI-compatible endpoint instead of a bespoke provider client.
- Fact ingestion: ev's `Fact` (blob payload with `fact_type`) is not the
  nexus FIH contract (`id: CoordId`, `content_hash`, `origin`, `content`,
  `creator`). Alignment is deferred while nexus is being reworked.
