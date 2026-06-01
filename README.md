# ev — ExaVerif

Exhaustive verification CLI for RISC-V custom instruction extensions.
Apache 2.0.

33.5 million combinations exhaustive in 32 seconds. 100% cross-validated
against Spike RISC-V simulation.

## What It Does

Given a YAML file describing instruction fields and constraints, ev generates
every valid combination, evaluates each against the constraint space, and reports
exactly which encodings are valid and which are not — deterministically and
exhaustively.

```
YAML → Domain expansion → Constraint evaluation → Projection → Report
```

A single command enumerates and evaluates 33.5 million combinations against the
actual CVA6 CV-X-IF coprocessor specification — the same encoding tables used
in OpenHW's own verification suite — and produces the result in 32 seconds:

```bash
ev verify --target tests/fixtures/cva6_xif_ref.xif.yaml
```

Output:
```
target: cva6_xif_ref
total:  33554432
passed: 196608
failed: 33357824
```

Every valid encoding is also verifiable through actual RISC-V simulation via
`ev simulate`, which packs all 196,608 valid encodings into a single ELF binary
and runs it under Spike:

```bash
EV_SIM_BACKEND=spike ev simulate --target tests/fixtures/cva6_xif_ref.xif.yaml
```

All 196,608 pass — the static constraint model and the RISC-V simulator
agree exactly.

## Quick Start

```bash
./run.sh                  # Full pipeline: auto-fix -> fmt -> clippy -> build -> test -> verify
./run.sh --demo           # Channel demo: cross-verify SSCCS POC golden anchors
./run.sh --code           # fmt -> clippy -> build -> test (strict)
./run.sh --verify         # Full verification including 33M combo fixture
```

Or step-by-step:

```bash
cargo build --release
ev verify --target tests/fixtures/all_pass.xif.yaml
ev verify --target tests/fixtures/sample.xif.yaml --json
ev synth --target tests/fixtures/all_pass.xif.yaml
ev simulate --target tests/fixtures/all_pass.xif.yaml
cargo test --release
```

## CLI Reference

```
ev verify    --target <file> [--json]    # Static constraint verification
ev simulate  --target <file> [--json]    # ISA simulation (Spike/QEMU/mock)
ev synth     --target <file> [--json]    # SystemVerilog generation + Yosys synthesis
```

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
    field_a: "operand_a"
    field_b: "operand_b"
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

### Built-in types

**Constraints**: `range`, `even`, `eq`, `neq`, `lt`, `gt`, `le`, `ge`,
`oneof`, `cross`.

**Projectors**: `sum`, `identity`, `parity`.

All types are extensible via `ConstraintRegistry` and `ProjectorRegistry`.

## Real-World Fixtures

| File | Based on | Combinations |
|------|----------|-------------|
| `cva6_xif_ref.xif.yaml` | CVA6 CV-X-IF coprocessor (actual RTL + verification suite) | 33,554,432 (196,608 valid) |
| `cva6_xif_mac.xif.yaml` | CVA6 XIF multiply-accumulate accelerator | 32,768 |
| `ibex_alu_ext.xif.yaml` | Ibex custom ALU extension | 512 |
| `rv32i_csr_access.xif.yaml` | Ibex-like CSR encoding | 4,608 |
| `all_pass.xif.yaml` | Simple ALU (no constraints) | 1,024 |
| `sample.xif.yaml` | Mixed pass/fail demo | 96 |

## Validation Results

| Metric | Value |
|--------|-------|
| Raw combinations evaluated | 33,554,432 |
| Valid encodings identified | 196,608 |
| Execution time (M1 Max) | 32 seconds |
| Spike cross-validation | 196,608 / 196,608 passed |
| Constraint types | 10 (range, even, eq, neq, lt, gt, le, ge, oneof, cross) |
| Simulation backends | Mock (default), Spike (EV_SIM_BACKEND=spike) |

## Architecture

```
src/
  main.rs           CLI (clap: verify, simulate, synth)
  spec.rs           VerificationSpec, FieldSpec, ConstraintSpec, ProjectorSpec
  compose.rs        Domain expansion (cartesian product with overflow guard)
  evaluate.rs       Constraint evaluation + projection
  registry.rs       ConstraintRegistry + ProjectorRegistry (pluggable builder)
  reporter.rs       ReporterCapable trait + TextReporter + JsonReporter
  format.rs         FormatCapable trait
  xif.rs            YamlFormat — XIF format parser
  fih.rs            Fact envelope (Vec<u8> blob, no embedded schema)
  synth/
    mod.rs          SvGenerator, MockSynthesisBackend, RunSynthesis
    sim.rs          RunSimulation trait + MockSimBackend
    backends/       SpikeBackend, YosysBackend
tests/
  fixtures/        7 YAML fixture files
scripts/
  demo-ssccs-poc.sh   Channel demo
```

Backends are pluggable via environment variables (Nexus-style capability trait):

| Variable | Values | Effect |
|----------|--------|--------|
| `EV_SIM_BACKEND` | `mock` (default), `spike` | Simulation backend |
| `EV_SYNTH_BACKEND` | `mock`, `yosys` (default) | Synthesis backend |
| `EV_SPIKE_BIN` | path | Spike binary location |
| `EV_PK_PATH` | path | Proxy kernel for Spike |
| `EV_RISCV_CC` | command | RISC-V cross-compiler |

## Prerequisites

- Rust 1.85+ ([rustup](https://rustup.rs/))
- Python 3 (for channel demo golden anchor parsing)
- Yosys (optional, for synthesis)
- Spike, riscv64-unknown-elf-gcc, riscv-pk (optional, for simulation)

## License

Apache 2.0 — see [LICENSE](LICENSE).
