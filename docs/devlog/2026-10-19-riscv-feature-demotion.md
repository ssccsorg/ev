# The generic build is the default, and RISC-V is one feature

Issue #70 raised the engine to a general space verifier and moved the RISC-V
layouts and the Spike backend behind the `riscv` feature. The default feature
set still contained `riscv`, so the code had the seam while the shipped
binary, `cargo test`, and `run.sh` did not exercise the generic side. The
layering was half-applied: a generic build existed, but it was an exception
that the suite did not support.

## What moved

| Surface | Before | After |
|---|---|---|
| `Cargo.toml` | `default = ["riscv"]` | `default = []` |
| Default `cargo build` | RISC-V profiles and Spike | generic engine |
| `cargo test` | all 163 tests | 152 tests, the 11 profile cases gated |
| `run.sh --code` | one feature state | generic and `--features riscv` |
| `run.sh --verify` | accepted any binary | rejects a generic binary and names the rebuild |
| Coverage | default features | `--features riscv` |
| Bench startup | loaded CVA6 fixtures unconditionally | loads them only with the feature |
| CLI | no layer surface | `ev capabilities` |

The feature gates in the tests are not a reduction in coverage. The RISC-V
suite still runs every case; the generic suite runs the cases that do not
select a profile. `tests/cli_test.rs`, `tests/structural_enum.rs`, and
`tests/cva6_derivation.rs` carry the gates.

## The CLI surface

`ev capabilities` prints the layers the binary actually carries:

```text
engine:      ev 0.1.0
formats:     xif
classifiers: structural, naive
reporters:   text, json, csv, trace
profiles:    (none)
simulation:  mock
synthesis:   yosys, mock
```

The same build with `--features riscv` adds `riscv-r`, `riscv-r4`, `r4`, and
`spike`. `--json` emits the same data. A generic build that parses a spec
naming `riscv-r` now reports the rebuild rather than only the absence:

```text
unknown profile `riscv-r`: this build has no layout profiles
(rebuild with `--features riscv` for the RISC-V profiles)
```

The command does not add a new record field. It reports the layer stack, and
the record contract stays what issue #66 defined.

## What did not change

No constraint type, projector, enumeration strategy, fixture count, or record
shape changed. The RISC-V build still classifies the same spaces and produces
the same records. The generic build classifies the same spaces for every spec
that declares no profile.

## Verification

```bash
cargo test --release                                   # 152 passed
cargo test --release --features riscv                  # 163 passed
cargo clippy --all-targets -- -D warnings              # clean
cargo clippy --all-targets --features riscv -- -D warnings  # clean
cargo bench --bench bench -- "struct_enum/ibex"        # generic build runs it
bash run.sh --code                                     # both states, exit 0
bash run.sh --verify                                   # RISC-V binary, exit 0
```

`run.sh --code` leaves the RISC-V binary in `target/release/ev`, and
`run.sh --verify` checks for the profile and the backend before it runs a
fixture.

---

## References

- ev issue #70, issue #67 (the capability seam)
- `Cargo.toml`, `src/main.rs`, `src/lib.rs`, `src/profile/mod.rs`
- `tests/cli_test.rs`, `tests/structural_enum.rs`, `tests/cva6_derivation.rs`
- `run.sh`, `scripts/coverage.sh`, `benches/bench.rs`
