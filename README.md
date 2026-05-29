# ev — ExaVerif

Exhaustive verification CLI for RISC‑V custom instruction extensions.
Apache 2.0.

## What It Does

Given a YAML file describing instruction fields and constraints, `ev` generates
every valid combination, checks them against the constraint space, and reports
which pass and which fail — deterministically.

```
YAML → Domain Expansion → Field.build() → observe() → Report
```

## Quick Start

```bash
./run.sh                  # Full pipeline: auto-fix → code → verify
./run.sh --demo           # Channel demo: cross-verify SSCCS POC golden anchors
./run.sh --code           # fmt → clippy → build → test (strict)
```

Or step-by-step:

```bash
cargo build --release
ev verify --target tests/fixtures/all_pass.xif.yaml
ev verify --target tests/fixtures/sample.xif.yaml --json
cargo test --release
```

## Input Format

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

Optional cross-field constraints reference fields by name (not axis index):

```yaml
constraints:
  - type: eq
    field_a: "operand_a"
    field_b: "operand_b"
```

Built-in constraint types: `range`, `even`, `eq`, `neq`, `lt`, `gt`, `le`, `ge`, `oneof`.
Built-in projector types: `sum`, `identity`, `parity`.
Extensible via `ConstraintRegistry` and `ProjectorRegistry`.

## Channel Demo

`./run.sh --demo` clones `ssccsorg/ssccs`, extracts golden anchors from
hand-written RISC‑V assembly (`observe_full.S`), and independently verifies
them through ev's exhaustive constraint engine:

```
narrow:   even ∧ range_0_10  →  2,REJECT,REJECT,10,REJECT  ✓
broad:    no constraints     →  2,3,5,10,12                ✓
sum3d_a:  (2,1,0)            →  3                          ✓
sum3d_b:  (1,2,3)            →  6                          ✓
parity:   {2,3}              →  0,1                        ✓
```

Same constraints, same results — two completely independent paths: handwritten
RISC‑V assembly vs. Rust-based exhaustive verification.

## Architecture

```
src/
  main.rs         CLI (clap: check, certify)
  spec.rs         VerificationSpec — format-agnostic internal IR
  format.rs       FormatCapable trait
  xif.rs          YamlFormat — RISCV-CTG-compatible YAML parser
  compose.rs      Domain expansion (cartesian product)
  evaluate.rs     Field construction + observe() pipeline
  registry.rs     ConstraintRegistry + ProjectorRegistry (pluggable)
  reporter.rs     ReporterCapable trait + TextReporter + JsonReporter
tests/
  fixtures/       all_pass.xif.yaml, sample.xif.yaml
scripts/
  demo-poc.sh     Channel demo: ev ↔ SSCCS POC golden anchor verification
```

Capability-trait architecture (same pattern as Nexus):

| Extension point | How to add |
|:---|:---|
| New constraint | `ConstraintRegistry::register("name", builder)` |
| New projector | `ProjectorRegistry::register("name", builder)` |
| New input format | Implement `FormatCapable` |
| New output format | Implement `ReporterCapable` |
| New channel | Add script to `scripts/` |

## Prerequisites

- Rust 1.85+ ([rustup](https://rustup.rs/))
- Python 3 (for channel demo golden anchor parsing)

## License

Apache 2.0 — see [LICENSE](LICENSE).
