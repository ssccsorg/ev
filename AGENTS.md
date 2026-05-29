# ev — ExaVerif: Agent Context Handoff

## Project Identity

Exhaustive verification CLI for RISC-V custom instruction extensions. Given a YAML spec describing instruction fields and constraints, `ev` generates every valid combination, evaluates constraints, and reports pass/fail — deterministically and exhaustively.

- Repository: `github.com/ssccsorg/ev`
- Language: Rust (edition 2021)
- License: Apache 2.0
- Current version: 0.1.0 (pre-1.0, not published to crates.io)

## Architecture

```
main.rs          CLI entry (clap: `verify`, `simulate`, `synth` subcommands)
lib.rs           Public re-exports
spec.rs          VerificationSpec, FieldSpec, ConstraintSpec, ProjectorSpec
compose.rs       Domain expansion (cartesian product with overflow guard)
evaluate.rs      Constraint evaluation + projection
registry.rs      ConstraintRegistry + ProjectorRegistry (pluggable builder pattern)
reporter.rs      ReporterCapable trait + TextReporter + JsonReporter
fih.rs           Fact envelope for neXus consumption
format.rs        FormatCapable trait
xif.rs           YamlFormat (XIF format parser, implements FormatCapable)
synth/mod.rs     GenerateRtl, RunSynthesis traits + SvGenerator + MockSynthesisBackend
synth/backends/  External synthesis backends (YosysBackend)
```

## Current State (as of May 2026, commit 547e2a9)

### Completed

- **Core pipeline**: YAML → expand_all → evaluate_all → TextReporter/JsonReporter
- **9 constraint types**: `range`, `even`, `eq`, `neq`, `lt`, `gt`, `le`, `ge`, `oneof` — all with serde deserialize, Check impl, SV assertion generation
- **3 projector types**: `sum`, `identity`, `parity`
- **Axis ordering fix**: Constraints reference fields by name (`field: "rs1"`) not numeric index — YAML declaration order independent
- **Overflow guard**: `MAX_COMBINATIONS = 10_000_000`, checked multiplication, `Result` return
- **Synthesis channel**: SV generation + YosysBackend + MockSynthesisBackend
- **Fact envelope**: `Fact` struct with type tags, timestamps, payload for neXus pipeline
- **CLI**: `ev verify` (static constraint), `ev simulate` (ISA simulation, WIP), `ev synth` (SV generation + synthesis)
- **69 tests** (56 lib + 13 CLI), all passing
- **Real RISC-V fixtures**: `ibex_alu_ext.xif.yaml` (456 combinations), `cva6_xif_mac.xif.yaml` (32,768 combinations)
- **CI**: fmt + clippy + test + verify (run.sh)

### Incomplete or Not Started

| Feature | Status | Notes |
|---------|--------|-------|
| **Spike backend integration** | Not started | `ev check --target spec.xif.yaml --spike` — feed each valid encoding to Spike. ssccs/poc already has Spike integration. Architecture: fork+exec spike per encoding (slow), or batch encodings into ELF. |
| **`--interpret` flag** | Not started | LLM-based failure explanation. Appears in CLI examples in docs. |
| **`certify` subcommand** | Not started | Mentioned in early docs but never implemented. |
| **crates.io publish** | Not started | `cargo publish` blocked by version 0.1.0 stability + docs completeness. |
| **Shared knowledge store** | Not started | Fact → R2/S3 → Nexus ingestion. |
| **Python/format channel** | Not started | `scripts/demo-poc.sh` exists but is not part of CI pipeline. |
| **Negative range end-to-end test** | Not started | `acc: [-128, 127]` with `projector: sum` JSON/text correctness. |
| **`sample.xif.yaml` update** | Not started | Old sample uses pre-axis-fix syntax. |
| **Non-JSON, non-text output** | Not started | e.g. CSV, trace format. |

### Known Issues / Future Work

- **constraint + field range overlap detection**: `oneof { field: "op", values: [0,1,2] }` with `op: { values: [0,1,2,3,4,5,6,7] }` is redundant. Could warn or auto-simplify.
- **Pipeline performance**: For huge specs (10M+ combinations), evaluation is O(N). No parallelization yet.
- **Spike batch mode**: For `--spike`, need to minimize process overhead.

## Test Infrastructure

```
cargo test                   # 56 lib tests + 13 CLI tests
cargo test --release         # slower but same tests
```

### Key fixture files in tests/fixtures/

| File | Purpose |
|------|---------|
| `all_pass.xif.yaml` | 1024 combos, all pass (no constraints) |
| `sample.xif.yaml` | Mixed pass/fail with `eq` constraint |
| `rv32i_csr_access.xif.yaml` | Ibex-like CSR encoding (3 fields + csr_addr) |
| `ibex_alu_ext.xif.yaml` | Ibex ALU: op_select/rs1/rd, oneof + neq, 456 pass |
| `cva6_xif_mac.xif.yaml` | CVA6 MAC: funct3/rs1/rs2/acc, neq, 18432 pass |
| `malformed_no_fields.xif.yaml` | Edge case: empty fields |
| `malformed_bad_type.xif.yaml` | Edge case: unknown constraint type |

## Git Branches

- `main`: stable, PR-merged
- `7-real-riscv-targets`: current work (Phase 1-3, PR #8 open)

## Key Design Decisions

1. **Capability traits**: `FormatCapable`, `ReporterCapable`, `GenerateRtl`, `RunSustain` — same pattern as Nexus. Adding new formats/backends requires zero changes to the pipeline.
2. **Field-name references**: All constraints reference fields by name string, not numeric axis. Solves BTreeMap ordering issue.
3. **ConstraintRegistry/ProjectorRegistry**: Builder pattern with `HashMap<String, fn(&ConstraintSpec, &HashMap<String,usize>) -> AnyCheck>`. Pluggable at runtime.
4. **No external tool coupling in lib**: Library crate has no dependency on Spike, Yosys, etc. CLI crate resolves backends via environment variables.

## How to Add

- **New constraint type**: (1) add variant to `ConstraintSpec` enum in `spec.rs`, (2) add builder closure in `ConstraintRegistry::default()` in `registry.rs`, (3) add `sv_constraint_assertion` arm in `synth/mod.rs`
- **New projector type**: (1) add variant to `ProjectorSpec` in `spec.rs`, (2) add builder in `ProjectorRegistry::default()` in `registry.rs`, (3) add `sv_projector` arm in `synth/mod.rs`
- **New input format**: implement `FormatCapable` trait
- **New output format**: implement `ReporterCapable` trait
