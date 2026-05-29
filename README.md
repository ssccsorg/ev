# ev — ExaVerif

Exhaustive verification CLI for RISC‑V custom instruction extensions.
Apache 2.0.

## What It Does

Given a YAML file describing instruction fields and constraints, `ev` generates
every valid combination, evaluates each against the constraint space, and reports
exactly which encodings are valid and which are not — deterministically and
exhaustively.

```
YAML → Domain expansion → Constraint evaluation → Projection → Report
```

ev treats every specification as a **Spec Space**: field domains define its axes,
constraints carve admissible subspaces, and each combination is a point within
this space. A single command enumerates and evaluates every point:

```bash
ev verify --target cva6_xif_ref.xif.yaml
```

Output:
```
target: cva6_xif_ref
total:  262144
passed: 3072
failed: 259072
```

The example above verifies the CVA6 CV-X-IF reference coprocessor against its
actual encoding specification. 3,072 valid encodings out of 262,144 possible —
the rest are correctly identified as illegal, not by random sampling but by
exhaustive enumeration. Every result is also available as structured JSON for
downstream consumption.

## Quick Start

```bash
./run.sh                  # Full pipeline: auto-fix → fmt → clippy → build → test → verify
./run.sh --demo           # Channel demo: cross-verify SSCCS POC golden anchors
./run.sh --check          # fmt + check only
```

Or step-by-step:

```bash
cargo build --release
ev verify --target tests/fixtures/all_pass.xif.yaml
ev verify --target tests/fixtures/sample.xif.yaml --json
ev synth --target tests/fixtures/all_pass.xif.yaml
cargo test --release
```

## CLI Reference

```
ev verify --target <file> [--json]    # Static constraint verification
ev synth  --target <file> [--json]    # SystemVerilog generation + synthesis
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

Optional cross-field constraints reference fields by name:

```yaml
constraints:
  - type: eq
    axis_a: 0
    axis_b: 1       # op_a must equal op_b
```

Cross constraint — map field_a values to allowed field_b sets:

```yaml
  - type: cross
    field_a: "funct3"
    field_b: "funct7"
    mapping:
      0: [0]
      1: [0, 1, 2, 3, 32]
```

### Built-in types

**Constraints**: `range`, `even`, `eq`, `neq`, `lt`, `gt`, `le`, `ge`,
`oneof`, `cross`.

**Projectors**: `sum`, `identity`, `parity`.

All types are extensible via `ConstraintRegistry` and `ProjectorRegistry`.

## Real-World Fixtures

| File | Based on | Combinations |
|------|----------|-------------|
| `cva6_xif_ref.xif.yaml` | CVA6 CV-X-IF reference coprocessor (RTL source) | 262,144 (3,072 valid) |
| `cva6_xif_mac.xif.yaml` | CVA6 XIF multiply-accumulate accelerator | 32,768 |
| `ibex_alu_ext.xif.yaml` | Ibex custom ALU extension | 512 |
| `rv32i_csr_access.xif.yaml` | Ibex-like CSR encoding | 3 × 32 × 32 × 16 |
| `all_pass.xif.yaml` | Simple ALU (no constraints) | 1,024 |
| `sample.xif.yaml` | Mixed pass/fail demo | 96 |

## Channel Demo

`./run.sh --demo` independently cross-verifies SSCCS POC golden anchors from
hand-written RISC‑V assembly through ev's exhaustive constraint engine:

```
narrow:   even ∧ range_0_10  →  2,REJECT,REJECT,10,REJECT  ✓
broad:    no constraints     →  2,3,5,10,12                ✓
sum3d_a:  (2,1,0)            →  3                          ✓
sum3d_b:  (1,2,3)            →  6                          ✓
parity:   {2,3}              →  0,1                        ✓
```

## Architecture

```
src/
  main.rs         CLI (clap: verify, synth)
  spec.rs         VerificationSpec, FieldSpec, ConstraintSpec, ProjectorSpec
  compose.rs      Domain expansion (cartesian product with overflow guard)
  evaluate.rs     Constraint evaluation + projection
  registry.rs     ConstraintRegistry + ProjectorRegistry (pluggable builder)
  reporter.rs     ReporterCapable trait + TextReporter + JsonReporter
  format.rs       FormatCapable trait
  xif.rs          YamlFormat — XIF format parser (implements FormatCapable)
  fih.rs          Fact envelope for neXus consumption
  synth/mod.rs    SvGenerator, MockSynthesisBackend, SynthesisMetrics
  synth/backends/ External synthesis backends (YosysBackend)
tests/
  fixtures/       7 YAML fixture files
scripts/
  demo-ssccs-poc.sh   Channel demo
```

Capability-trait architecture (same pattern as Nexus):

| Extension point | How to add |
|:---|:---|
| New constraint type | Add variant to `ConstraintSpec` in `spec.rs`, builder in `ConstraintRegistry::default()`, SV arm in `synth/mod.rs` |
| New projector type | Add variant to `ProjectorSpec` in `spec.rs`, builder in `ProjectorRegistry::default()`, SV arm in `synth/mod.rs` |
| New input format | Implement `FormatCapable` trait |
| New output format | Implement `ReporterCapable` trait |

## Prerequisites

- Rust 1.85+ ([rustup](https://rustup.rs/))
- Python 3 (for channel demo golden anchor parsing)
- Yosys (optional, for synthesis; falls back to Docker)

## License

Apache 2.0 — see [LICENSE](LICENSE).
