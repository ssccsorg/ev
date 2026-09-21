# ev — ExaVerif: Agent Context Handoff

## Project Identity

Exhaustive verification CLI for RISC-V custom instruction extensions. Given
a YAML spec describing instruction fields and constraints, `ev` generates
every structurally valid combination, evaluates the constraints, and reports
pass/fail deterministically and exhaustively.

- Repository: `github.com/ssccsorg/ev`
- Language: Rust (edition 2021), Apache 2.0
- Version: 0.1.0 (pre-1.0, not published to crates.io)

## Architecture

```text
src/
  main.rs           CLI (clap: verify, simulate, synth, fact decode)
  lib.rs            public re-exports
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
- CVA6 fixtures derive from the hardware decoder mask table
  (`cvxif_instr_pkg.sv`, `instr_decoder.sv`) at commit `6544a714c`.
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
cargo test --release          # 126 tests: 83 lib, 28 CLI, 8 structural,
                              # 5 tagma, 2 golden anchor. None ignored.
cargo bench -- cva6_full      # full-space CVA6 group
cargo bench -- struct_enum_validity   # correctness guard, must stay green
./run.sh                      # fmt, clippy, build, test, verify
./run.sh --verify             # Yosys, fixtures, golden anchors, Spike
./run.sh --demo               # channel demo: the ssccs POC assembly golden anchors
./run.sh --coverage           # coverage gate
```

| Fixture | Raw combinations | Valid |
|---|---:|---:|
| `cva6/xif_ref.xif.yaml` | 33,554,432 | 196,608 |
| `cva6/xif_ref_r4.xif.yaml` | 16,384 | 2,560 |
| `cva6/xif_mac.xif.yaml` | 32,768 | 28,672 |
| `cva6/xif_madd.xif.yaml` | 32,768 | 4,096 |
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

## How to Extend

- New constraint type: add a variant to `ConstraintSpec` in `spec/mod.rs`,
  a builder closure in `ConstraintRegistry::default()`, and an arm in
  `sv_constraint_assertion` in `synth/mod.rs`.
- New projector type: add a variant to `ProjectorSpec`, a builder in
  `ProjectorRegistry::default()`, and an arm in `sv_projector`.
- New input format: implement `FormatCapable`. New output format: implement
  `ReporterCapable` and pass the total through.

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

- Issue #44: execute the accepted CVA6 custom-3 encodings through the
  standard CVA6 tandem flow, which needs the external CVA6 repository.
- Issue #18: `--interpret` for failure explanation, re-scoped to an
  OpenAI-compatible endpoint instead of a bespoke provider client.
- Fact ingestion: ev's `Fact` (blob payload with `fact_type`) is not the
  nexus FIH contract (`id: CoordId`, `content_hash`, `origin`, `content`,
  `creator`). Alignment is deferred while nexus is being reworked.
