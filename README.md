# ev — ExaVerif

Exhaustive verification CLI for RISC-V custom instruction extensions.
Apache 2.0.

33.5 million combinations evaluated deterministically: 10.4 s with the
standard pipeline, 19.0 ms with the Tagma-based structural enumeration
(~550x, same-language baseline). The CVA6 fixtures are derived from the
hardware decoder mask table (commit `6544a714c`); the Spike backend
cross-checks the constraint model and instruction-word assembly in C.

## What It Does

Given a YAML file describing instruction fields and constraints, ev enumerates
and evaluates every valid combination, reports exactly which encodings are valid
and which are not — deterministically and exhaustively.

Constraint types are split between structural constraints (oneof, range, bitmask,
cross) that are encoded directly into the enumeration space, and runtime
constraints (eq, neq, lt, gt, le, ge, even) that are checked per combination.
Only structurally valid combinations are ever generated.

A single command enumerates and evaluates 33.5 million combinations against the
CVA6 CV-X-IF encoding space derived from the hardware decoder mask table
(`cva6/core/cvxif_example/include/cvxif_instr_pkg.sv` at commit `6544a714c`),
and produces the result in 19.0 milliseconds:

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
ev simulate  --target <file> [--format <fmt>]  # C/Rust recheck under Spike/mock
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

### Built-in types

**Constraints**: `range`, `even`, `eq`, `neq`, `lt`, `gt`, `le`, `ge`,
`oneof`, `cross`, `bitmask`, `enable_mask`, `enable_set`.

**Projectors**: `sum`, `identity`, `parity`.

All types are extensible via `ConstraintRegistry` and `ProjectorRegistry`.

## Real-World Fixtures

Valid counts below are the `evaluate_all` results on the committed fixtures
(release build).

| File | Based on | Combinations (raw) | Valid |
|------|----------|-------------------:|------:|
| `cva6/xif_ref.xif.yaml` | CVA6 CV-X-IF hardware decoder mask table (commit 6544a714c) | 33,554,432 | 196,608 |
| `cva6/xif_ref_r4.xif.yaml` | CVA6 CV-X-IF R4 format (func2 + rs3) | 16,384 | 2,560 |
| `cva6/xif_mac.xif.yaml` | CVA6 XIF multiply-accumulate | 32,768 | 28,672 |
| `cva6/xif_madd.xif.yaml` | CVA6 XIF madd/msub encoding | 32,768 | 4,096 |
| `cva6/xif_encoding.xif.yaml` | CVA6 XIF encoding-only (register-reduced) | 8,192 | 48 |
| `ibex/alu_ext.xif.yaml` | Ibex custom ALU extension | 524,288 | 4,096 |
| `ibex/csr_access.xif.yaml` | Ibex-like CSR encoding | 49,152 | 49,152 |
| `ibex/rv32imcb.xif.yaml` | Ibex RV32IMCB (ibex_decoder.sv) | 524,288 | 92,160 |
| `ibex/rv32imcb_imm.xif.yaml` | Ibex RV32IMCB I-type encoding | 65,536 | 55,616 |
| `common/all_pass.xif.yaml` | Simple ALU (no constraints) | 1,024 | 1,024 |
| `common/sample.xif.yaml` | Mixed pass/fail demo | 96 | 12 |

## Validation Results

| Metric | Value |
|--------|-------|
| Raw combinations evaluated (CVA6 full) | 33,554,432 |
| Valid combinations identified (CVA6 full) | 196,608 |
| Standard pipeline time (evaluate_all, release) | 10.4 s |
| Structural pipeline time (struct_enum, release) | 19.0 ms |
| Speedup (same-language baseline, this machine) | ~550x |
| Spike backend | C/Rust recheck: 196,608 / 196,608 agree |
| Constraint types | 13 (range, even, eq, neq, lt, gt, le, ge, oneof, cross, bitmask, enable_mask, enable_set) |
| Tests | 92 (73 lib + 14 CLI + 5 structural), all passing |
| Simulation backends | Mock (default), Spike (`EV_SIM_BACKEND=spike`) |

Benchmark methodology and reproducibility: both pipelines are Rust, the same
release profile, measured by criterion; the speedup is the O(N) vs O(V)
enumeration-strategy gain, not a language effect. Reproduce with
`cargo bench -- cva6_full` on the committed fixtures.

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
benches/
  bench.rs          Performance reference (fixtures, methodology, groups)
tests/
  fixtures/
    common/         4 YAML fixture files
    cva6/           5 YAML fixture files
    ibex/           4 YAML fixture files
  cli_test.rs       14 integration tests (+ 2 heavy CVA6 tests ignored by default)
  structural_enum.rs 5 structural enumeration regression tests
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
