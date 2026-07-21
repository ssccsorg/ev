# ev — ExaVerif

Exhaustive verification CLI for RISC-V custom instruction extensions.
Apache 2.0.

33.5 million combinations exhaustive in 31 milliseconds. 100% cross-validated
against Spike RISC-V simulation.

## What It Does

Given a YAML file describing instruction fields and constraints, ev enumerates
and evaluates every valid combination, reports exactly which encodings are valid
and which are not — deterministically and exhaustively.

Constraint types are split between structural constraints (oneof, range, bitmask,
cross) that are encoded directly into the enumeration space, and runtime
constraints (eq, neq, lt, gt, le, ge, even) that are checked per combination.
Only structurally valid combinations are ever generated.

A single command enumerates and evaluates 33.5 million combinations against the
actual CVA6 CV-X-IF coprocessor specification — the same encoding tables used
in OpenHW's own verification suite — and produces the result in 31 milliseconds:

```bash
ev verify --target tests/fixtures/cva6/xif_ref.xif.yaml
```

Output:
```
target: cva6_xif_ref
total:  33554432
passed: 229376
failed: 33325056
```

Every valid encoding is also verifiable through actual RISC-V simulation via
`ev simulate`, which packs all valid encodings into a single ELF binary and
runs it under Spike:

```bash
EV_SIM_BACKEND=spike ev simulate --target tests/fixtures/cva6/xif_ref.xif.yaml
```

All valid encodings pass — the static constraint model and the RISC-V simulator
agree exactly.

## Quick Start

```bash
./run.sh                  # Full pipeline: fmt -> clippy -> build -> test -> verify
./run.sh --demo           # Channel demo: cross-verify golden anchors
./run.sh --code           # fmt -> clippy -> build -> test (strict)
./run.sh --verify         # Full verification including 33M combo fixture
```

Or step-by-step:

```bash
cargo build --release
ev verify --target tests/fixtures/common/all_pass.xif.yaml
ev verify --target tests/fixtures/common/sample.xif.yaml --json
ev synth --target tests/fixtures/common/all_pass.xif.yaml
ev simulate --target tests/fixtures/common/all_pass.xif.yaml
cargo test --release
```

## CLI Reference

```
ev verify    --target <file> [--format <fmt>]  # Static constraint verification
ev simulate  --target <file> [--format <fmt>]  # ISA simulation (Spike/mock)
ev synth     --target <file> [--json]          # SystemVerilog + Yosys synthesis
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

Cross constraint — map field_a values to allowed field_b sets:

```yaml
  - type: cross
    field_a: "funct3"
    field_b: "funct7"
    mapping:
      0: [2, 6, 8, 32]
      1: [0]
      2: [96]
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

### Built-in types

**Constraints**: `range`, `even`, `eq`, `neq`, `lt`, `gt`, `le`, `ge`,
`oneof`, `cross`, `bitmask`, `enable_mask`, `enable_set`.

**Projectors**: `sum`, `identity`, `parity`.

All types are extensible via `ConstraintRegistry` and `ProjectorRegistry`.

## Real-World Fixtures

| File | Based on | Combinations (raw) | Valid |
|------|----------|-------------------:|------:|
| `cva6/xif_ref.xif.yaml` | CVA6 CV-X-IF coprocessor (actual RTL) | 33,554,432 | 229,376 |
| `cva6/xif_ref_r4.xif.yaml` | CVA6 CV-X-IF R4 format (func2+rs3) | 2,097,152 | 1,280 |
| `cva6/xif_mac.xif.yaml` | CVA6 XIF multiply-accumulate | 32,768 | 18,432 |
| `cva6/xif_madd.xif.yaml` | CVA6 XIF madd/msub encoding | — | — |
| `cva6/xif_encoding.xif.yaml` | CVA6 XIF encoding-only (register-reduced) | 8,192 | 48 |
| `ibex/alu_ext.xif.yaml` | Ibex custom ALU extension | 524,288 | 4,096 |
| `ibex/csr_access.xif.yaml` | Ibex-like CSR encoding | 4,608 | 4,608 |
| `ibex/rv32imcb.xif.yaml` | Ibex RV32IMCB (ibex_decoder.sv) | 524,288 | 313,344 |
| `ibex/rv32imcb_imm.xif.yaml` | Ibex RV32IMCB I-type encoding | 65,536 | 55,616 |
| `common/all_pass.xif.yaml` | Simple ALU (no constraints) | 1,024 | 1,024 |
| `common/sample.xif.yaml` | Mixed pass/fail demo | 96 | 12 |

## Validation Results

| Metric | Value |
|--------|-------|
| Raw combinations evaluated | 33,554,432 |
| Valid encodings identified | 229,376 |
| Execution time (M1 Max) | 31 milliseconds |
| Spike cross-validation | 100% agreement |
| Constraint types | 13 (range, even, eq, neq, lt, gt, le, ge, oneof, cross, bitmask, enable_mask, enable_set) |
| Tests | 89 (73 lib + 16 CLI), all passing |
| Simulation backends | Mock (default), Spike (`EV_SIM_BACKEND=spike`) |

## Architecture

```
src/
  main.rs           CLI (clap: verify, simulate, synth)
  spec/             VerificationSpec, FieldSpec, ConstraintSpec, ProjectorSpec
  verify/
    compose.rs      Domain expansion + structural enumeration
    evaluate.rs     Constraint evaluation + projection + CoordSpace validation
    registry.rs     ConstraintRegistry + ProjectorRegistry (pluggable builder)
  report/
    reporter.rs     ReporterCapable trait + TextReporter + CsvReporter
                    JsonReporter + TraceReporter
    fih.rs          Fact envelope (typed, timestamped, content-addressed)
  format/
    xif.rs          YamlFormat — XIF format parser
  synth/
    mod.rs          SvGenerator, MockSynthesisBackend, RunSynthesis
    sim.rs          RunSimulation trait + MockSimBackend
    backends/       SpikeBackend, YosysBackend
tests/
  fixtures/
    common/         4 YAML fixture files
    cva6/           5 YAML fixture files
    ibex/           4 YAML fixture files
  cli_test.rs       16 integration tests
```

Backends are pluggable via environment variables:

| Variable | Values | Effect |
|----------|--------|--------|
| `EV_SIM_BACKEND` | `mock` (default), `spike` | Simulation backend |
| `EV_SYNTH_BACKEND` | `mock`, `yosys` (default) | Synthesis backend |
| `EV_SPIKE_BIN` | path | Spike binary location |
| `EV_PK_PATH` | path | Proxy kernel for Spike |
| `EV_RISCV_CC` | command | RISC-V cross-compiler |

## Prerequisites

- Rust 1.85+ ([rustup](https://rustup.rs/))
- Python 3 (for channel demo)
- Yosys (optional, for synthesis)
- Spike, riscv64-unknown-elf-gcc, riscv-pk (optional, for simulation)

## License

Apache 2.0 — see [LICENSE](LICENSE).
