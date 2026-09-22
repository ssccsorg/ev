# ev — ExaVerif

Exhaustive verification CLI for RISC-V custom instruction extensions.
Apache 2.0.

ev verifies instruction encoding spaces, and a target is described by its
spec alone: the fields, their bit positions in the word, and the constraints
between them. A run classifies every point of that space and nothing else, and
identical runs produce identical records; there is no sampling and no
probabilistic claim. No constraint type or projector carries a target's name
or constant. The cores and designs that appear here, CVA6, Ibex, and the Tagma
decoder, are samples that exercise the engine and its channels, and each one
is derived from that project's own source rather than restated from memory.

33.5 million combinations verified deterministically in about 0.2 s (release)
through the structural enumeration pipeline, which is the CLI default since
issue #42. The CVA6 fixtures are derived from the hardware decoder mask
table (`6544a714c`), and `tests/cva6_derivation.rs` re-derives the three of
them that cite the table against a committed extraction of it; the Spike
backend cross-checks the constraint model and instruction-word assembly in C.

## What It Does

Given a YAML file describing instruction fields and constraints, ev enumerates
and evaluates every valid combination, reports exactly which encodings are valid
and which are not — deterministically and exhaustively.

Constraint types are split between structural constraints (oneof, range, bitmask,
cross, enable_mask, enable_set) that are encoded directly into the enumeration
space, and runtime constraints (eq, neq, lt, gt, le, ge, even) that are checked
per combination. Only structurally valid combinations are ever generated.

A single command verifies the 33.5 million combination CVA6 CV-X-IF encoding
space derived from the hardware decoder mask table
(`cva6/core/cvxif_example/include/cvxif_instr_pkg.sv` at commit `6544a714c`)
in about 0.2 seconds, by enumerating only the structurally valid
combinations:

```bash
ev verify --target tests/fixtures/cva6/xif_ref.xif.yaml
```

Output:

```
target: cva6_xif_ref
total:  33554432
passed: 196608
failed: 33357824
```

Every valid combination is also cross-checked by a C reimplementation via
`ev simulate`, which packs all valid combinations into a single ELF binary and
runs it under Spike + pk:

```bash
EV_SIM_BACKEND=spike ev simulate --target tests/fixtures/cva6/xif_ref.xif.yaml
```

All 196,608 rows agree between the C and Rust implementations of the constraint
model and of the instruction-word assembly. This is not ISA-level execution:
custom-3 opcodes are illegal in the base RISC-V ISA (which is why CVA6 offloads
them), so Spike never executes them.

## Quick Start

```bash
./run.sh                  # Full pipeline: fmt -> clippy -> build -> test -> verify
./run.sh --demo           # Channel demo: cross-verify golden anchors
./run.sh --code           # fmt -> clippy -> build -> test (strict)
./run.sh --verify         # Full verification including 33M combo fixture
./run.sh --coverage       # Code coverage gate (cargo-llvm-cov, 80% thresholds)
```

Or step-by-step:

```bash
cargo build --release
ev verify --target tests/fixtures/common/all_pass.xif.yaml
ev verify --target tests/fixtures/common/sample.xif.yaml --json
ev synth --target tests/fixtures/common/all_pass.xif.yaml
ev synth --design tests/fixtures/rtl/decode_demo.v --top decode_demo
ev simulate --target tests/fixtures/common/all_pass.xif.yaml
cargo test --release
```

## CLI Reference

```
ev verify    --target <file> [--format <fmt>]  # Static constraint verification
ev simulate  --target <file> [--format <fmt>]  # C/Rust recheck under Spike/mock
ev synth     --target <file> [--json]          # Generate RTL from a spec, then synthesize it
ev synth     --design <file> [--top <mod>]     # Synthesize an RTL file directly
ev fact decode                                  # Decode Fact JSON from stdin
```

Output formats: `text` (default), `json`, `csv`, `trace`.

## Input Format

### Field specification

```yaml
target: simple_alu
fields:
  op_a:
    range: [0, 15]
  op_b:
    range: [0, 15]
  op_code:
    values: [0, 1, 2, 3]
projector:
  type: sum
```

### Constraints

Cross-field constraints reference fields by name:

```yaml
constraints:
  - type: eq
    field_a: "rs1"
    field_b: "rs2"
```

Cross constraint — map field_a values to allowed field_b sets. The example is
the CVA6 decoder-derived mapping:

```yaml
  - type: cross
    field_a: "funct3"
    field_b: "funct7"
    mapping:
      0: [0]
      1: [0, 1, 2, 3, 4]
```

Bitmask constraint — field bits matching a pattern:

```yaml
  - type: bitmask
    field: "funct7"
    mask: 2
    value: 2
```

Conditional field activation — force fields to zero when trigger matches:

```yaml
  - type: enable_mask
    field: "funct3"
    value: 1
    disable: ["rs1", "rs2", "rd"]
```

Conditional field assignment — set fields to specified values on trigger:

```yaml
  - type: enable_set
    field: "op"
    value: 0
    set:
      - { field: "rd", value: 0 }
      - { field: "rs1", value: 5 }
```

The decompose projector splits one field into packed mixed-radix axes, each
at its own bit range. The syntagma anchor layout
`offset[28:15] i[14:10] m[9:5] f[4:0]` is one instance:

```yaml
projector:
  type: decompose
  field: "code"
  base: 0xAC00
  offset_shift: 15
  axes:
    - { radix: 28, width: 5, shift: 0 }   # f = offset % 28
    - { radix: 21, width: 5, shift: 5 }   # m = (offset / 28) % 21
    - { width: 5, shift: 10 }             # i = offset / 588
```

### Built-in types

**Constraints**: `range`, `even`, `eq`, `neq`, `lt`, `gt`, `le`, `ge`,
`oneof`, `cross`, `bitmask`, `enable_mask`, `enable_set`.

**Projectors**: `sum`, `identity`, `parity`, `decompose`.

All types are extensible via `ConstraintRegistry` and `ProjectorRegistry`.

## Fixture Provenance

Every spec under `tests/fixtures/` is listed with its source, how it was
derived, and what keeps it honest. Counts are the `evaluate_all` results on the
committed fixtures in a release build.

| Fixture | Source and revision | Derivation | Raw | Valid | Gate |
|---------|---------------------|------------|----:|------:|------|
| `cva6/xif_ref.xif.yaml` | CVA6 `core/cvxif_example/include/cvxif_instr_pkg.sv` and `instr_decoder.sv` at `6544a714c` | Hardware decoder mask table, R-type field terms | 33,554,432 | 196,608 | Derivation gate, count assertion, structural equivalence, CLI counts, Spike recheck |
| `cva6/xif_ref_r4.xif.yaml` | The same table and revision | The same table in R4 field terms (`func2`, `rs3`) | 16,384 | 2,560 | Derivation gate, count assertion, structural equivalence, Spike recheck |
| `cva6/xif_madd.xif.yaml` | The same table and revision, the MADD-family entries (`0x43`, `0x47`, `0x4B`, `0x4F`) | Mask table re-expressed as R4 fields | 32,768 | 1,024 | Derivation gate, count assertion |
| `cva6/xif_encoding.xif.yaml` | The same table, plus `verif/env/corev-dv/custom/cvxif_custom_instr.sv`, at `6544a714c` | DV-class encodings on a register-reduced space, including encodings the decoder rejects | 8,192 | 48 | Count assertion, Spike recheck |
| `cva6/xif_mac.xif.yaml` | None | Hand-written multiply-accumulate accelerator model | 32,768 | 28,672 | Count assertion, CLI output |
| `ibex/rv32imcb.xif.yaml` | Ibex `rtl/ibex_decoder.sv`, `OPCODE_OP`, RV32BFull and RV32MFast, at `f4540774` | Hand transcription of the decoder case statement; not compared with the decoder by a gate (issue #62) | 524,288 | 92,160 | Count assertion, source pin |
| `ibex/rv32imcb_imm.xif.yaml` | Ibex `rtl/ibex_decoder.sv`, `OPCODE_OP_IMM`, RV32BFull, at `f4540774` | Hand transcription of the decoder case statement; not compared with the decoder by a gate (issue #62) | 65,536 | 55,616 | Count assertion, source pin |
| `ibex/csr_access.xif.yaml` | The RISC-V Zicsr specification | Hand-written standard encoding domain, with no illegal combination | 49,152 | 49,152 | Count assertion, CLI output |
| `tagma/tagma_decoder.xif.yaml` | syntagma `hw/rtl/tagma_decoder.v` and the Tagma whitepaper (`10.5281/zenodo.21302508`); the reference engine is `tagma-core` at `205e3a0` | Input-domain contract of the decoder, projected into the golden-anchor layout | 65,536 | 11,172 | Count assertion, Tagma fixture tests, golden anchor cross-channel |
| `tagma/tagma_demo_top.xif.yaml` | syntagma `hw/rtl/tagma_demo_top.v` | Registered output space of the three decoder axes | 11,172 | 11,172 | Count assertion, Tagma fixture tests |
| `common/all_pass.xif.yaml` | None | Synthetic simple ALU with no constraints | 1,024 | 1,024 | Count assertion, CLI tests, synthesis channel, Spike recheck |
| `common/sample.xif.yaml` | None | Synthetic mixed pass/fail demonstration | 96 | 12 | Count assertion, structural equivalence, CLI output and JSON parse, Spike recheck |
| `common/enable_mask_demo.xif.yaml` | None | Synthetic `oneof` + `cross` + `enable_mask` coverage with an `identity` projector | 524,288 | 4,096 | Count assertion, structural parity on the distinct passing set |
| `common/malformed_no_fields.xif.yaml` | None | Parser negative: a spec with no fields | 0 | 0 | CLI test: exits zero with zero counts |
| `common/malformed_bad_type.xif.yaml` | None | Parser negative: an unknown constraint type | n/a | n/a | CLI test: exits non-zero and names the type |
| `common/malformed_decompose.xif.yaml` | None | Parser negative: a projector whose axes overlap | n/a | n/a | CLI test: exits non-zero and names the axis |

The gate names above: a derivation gate re-reads the fixture's source and
compares the accepted set with it (`tests/cva6_derivation.rs`), a source pin
compares a checkout with a committed pin without deriving anything
(`tests/ibex_source_pin.rs`), a count assertion is a `_verify_check` line in
`run.sh`, structural equivalence and structural parity are
`tests/structural_enum.rs`, CLI counts and CLI output are `tests/cli_test.rs`,
Tagma fixture tests are `tests/tagma_fixture.rs`, and the golden anchor
cross-channel is `tests/golden_anchor.rs`. The Spike recheck is `ev simulate`
with the C backend: `run.sh` runs it on the register-reduced fixtures and it is
available for any fixture on the command line.

Four artifacts under `tests/fixtures/` are inputs to channels rather than
specs:

| Artifact | Origin | Gate |
|----------|--------|------|
| `cva6/mask_table.json` | Extraction of the CVA6 decoder mask table at `6544a714c`, carrying the source path and the file digests | Re-extracted and compared by `tests/cva6_derivation.rs` when `CVA6_DIR` is set |
| `ibex/decoder_pin.json` | The decoder file, revision, digest, and assumed configuration the two RV32IMCB fixtures were transcribed from, plus the fixture files and digests it covers | Compared with a checkout by `tests/ibex_source_pin.rs` when `IBEX_DIR` is set; the fixture digests are checked wherever the tests run |
| `rtl/decode_demo.v` | Hand-written demo decoder for the design-only synthesis path | `run.sh --verify` requires a populated gate count |
| `yosys/stat_decode_demo.json` | Captured Yosys 0.65 `stat -json` report for `decode_demo.v` | `parse_stat_reads_a_captured_report` |

One limitation is recorded here rather than smoothed over. The CVA6 fixtures are
re-derived from their source by a gate, and the Ibex pair is pinned to its
decoder, but no gate compares the Ibex mapping with the decoder that states it:
that decoder is behavioural code rather than a table, so deriving the accepted
pairs from it is its own subject (issue #62).

## Validation Results

| Metric | Value |
|--------|-------|
| Raw combinations verified (CVA6 full) | 33,554,432 (counted, not enumerated) |
| Valid combinations identified (CVA6 full) | 196,608 |
| CLI verify time (CVA6 full, structural pipeline, release) | ~0.2 s end-to-end (core pipeline 36 ms benched) |
| Previous CLI time (expand_all, release) | 13.1 s (benched evaluate) |
| struct_enum benchmark (same machine, release) | 18.8 ms |
| CVA6 fixture derivation | the three decoder-derived fixtures match a re-extraction of the mask table at `6544a714c`: 6 `(funct3, funct7)` pairs for `xif_ref`, 5 `(funct3, func2)` pairs for `xif_ref_r4`, 1 for `xif_madd` |
| Ibex source pin | the two RV32IMCB fixtures are pinned to `ibex_decoder.sv` at `f4540774` (digest `ba04566d`), compared when `IBEX_DIR` is set; the mapping itself is not yet derived from the decoder (issue #62) |
| Spike backend | C/Rust recheck: 196,608 / 196,608 agree |
| Tagma decoder cross-channel | 11,172 / 11,172 projections equal `tagma_core::Coord::to_axes`; the generated `golden_anchors.hex` matches line by line when `EV_TAGMA_ANCHORS` is set |
| Synthesis channel | `--design` on the syntagma Tagma decoder reports 478 cells, the number the syntagma generic Yosys flow reports for the same RTL; `--target` on `all_pass` reports 28 (both with Yosys 0.65) |
| SSCCS POC channel demo (`./run.sh --demo`, needs an ssccs checkout) | 5 / 5 channels match the hand-written assembly golden anchors |
| Constraint types | 13 (range, even, eq, neq, lt, gt, le, ge, oneof, cross, bitmask, enable_mask, enable_set) |
| Projector types | 4 (sum, identity, parity, decompose) |
| Tests | 132 (83 lib + 28 CLI + 8 structural + 5 tagma + 2 golden anchor + 4 derivation + 2 source pin), all passing, none ignored |
| Coverage gate | 80% lines / 80% regions (llvm-cov, all modules incl. Spike/Yosys backends) |
| Simulation backends | Mock (default), Spike (`EV_SIM_BACKEND=spike`) |

Benchmark methodology and reproducibility: the structural pipeline is the
CLI default; the raw total is computed from the field domains without
enumeration, and only structurally valid combinations are generated and
evaluated. Reproduce with `cargo bench -- cva6_full` on the committed
fixtures.

## Architecture

```
src/
  main.rs           CLI (clap: verify, simulate, synth, fact decode)
  classification/   Verdict, Classification — the engine's output type
  spec/             VerificationSpec, FieldSpec, ConstraintSpec, ProjectorSpec,
                    EncodingLayout, FieldBitMapping
  verify/
    compose.rs      Domain expansion, raw_total_combinations, StructuralEnum
    evaluate.rs     evaluate_all, evaluate_structural, validate_into_space,
                    build_runtime_checks
    registry.rs     ConstraintRegistry, ProjectorRegistry, Check/Evaluator traits
  report/
    reporter.rs     ReporterCapable trait + Text/Csv/Json/Trace reporters
    fih.rs          Fact envelope (fact_type, origin, target, payload,
                    timestamp, parent_fact_id)
  format/
    mod.rs          FormatCapable trait
    xif.rs          YamlFormat, the XIF parser
  synth/
    mod.rs          GenerateRtl, RunSynthesis, SvGenerator, MockSynthesisBackend
    sim.rs          RunSimulation trait + MockSimBackend
    backends/       SpikeBackend, YosysBackend
benches/
  bench.rs          Performance reference (fixtures, methodology, groups)
tests/
  fixtures/
    common/         6 YAML fixture files
    cva6/           5 YAML fixture files, `mask_table.json` (the committed decoder extraction)
    ibex/           3 YAML fixture files, `decoder_pin.json` (the decoder source pin)
    tagma/          2 YAML fixture files
    rtl/            1 Verilog design fixture
    yosys/          1 captured Yosys stat report
  cli_test.rs       28 integration tests
  structural_enum.rs 8 structural enumeration regression tests
  tagma_fixture.rs  5 Tagma fixture tests
  golden_anchor.rs  2 tagma golden anchor cross-channel tests
  cva6_derivation.rs 4 CVA6 fixture derivation gate tests
  ibex_source_pin.rs 2 Ibex decoder source pin tests
```

Backends are pluggable via environment variables:

| Variable | Values | Effect |
|----------|--------|--------|
| `EV_SIM_BACKEND` | `mock` (default), `spike` | Simulation backend |
| `EV_SYNTH_BACKEND` | `mock`, `yosys` (default) | Synthesis backend |
| `EV_SPIKE_BIN` | path | Spike binary location |
| `EV_PK_PATH` | path | Proxy kernel for Spike |
| `EV_RISCV_CC` | command | RISC-V cross-compiler |
| `EV_TAGMA_ANCHORS` | path | Generated `hw/rtl/golden_anchors.hex` for the tagma artifact channel |
| `SYNTAGMA_DIR` | path | Sibling syntagma checkout (default `../syntagma`), the artifact-channel fallback |
| `CVA6_DIR` | path | Sibling CVA6 checkout (default `../cva6`) for the fixture derivation gate's source channel |
| `IBEX_DIR` | path | Sibling Ibex checkout (default `../ibex`) for the source pin's checkout channel |
| `EV_UPDATE_MASK_TABLE` | `1` | Rewrite `tests/fixtures/cva6/mask_table.json` from a checkout at the pinned commit |

## Prerequisites

- Rust 1.85+ ([rustup](https://rustup.rs/))
- cargo-llvm-cov (optional, for `--coverage`; needs the `llvm-tools-preview` rustup component)
- Python 3 (for channel demo)
- Yosys (optional, for synthesis)
- Spike, riscv64-unknown-elf-gcc, riscv-pk (optional, for simulation)

## License

Apache 2.0 — see [LICENSE](LICENSE).
