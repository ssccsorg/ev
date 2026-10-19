# RISC-V is one target module in the default set

Issue #70 raised the engine to a general space verifier and moved the RISC-V
layouts and the Spike backend behind the `riscv` feature. The default feature
set still contained `riscv`, so the code had the seam while the build did not
distinguish the engine from the target modules that exist. The first pass
moved `default` to `[]`, which over-corrected: it made the generic core the
default and left the target set empty. The default build should include one or
more target modules, and RISC-V is the only one that exists today.

## What moved

| Surface | Before | After |
|---|---|---|
| `Cargo.toml` | `default = ["riscv"]` | `default = ["targets"]`, `targets = ["riscv"]`, `riscv = []` |
| Default `cargo build` | RISC-V profiles and Spike, with no name for the set | the target set, currently RISC-V |
| Generic core | `--no-default-features` | `--no-default-features` |
| `cargo test` | all 163 tests | 163 in the default set, 152 in the generic core |
| `run.sh --code` | one feature state | generic core, default targets, explicit `riscv` |
| `run.sh --verify` | accepted any binary | rejects a binary without the RISC-V module |
| Coverage | default features | the default target set |
| Bench startup | loaded CVA6 fixtures unconditionally | loads them only with the target module |
| CLI | no layer surface | `ev capabilities` reports target modules and layers |

The feature gates in the tests are not a reduction in coverage. The default
suite runs 163 tests, and the generic core runs 152. The explicit
`--no-default-features --features riscv` selection also runs 163, which proves
the module composes outside the aggregate. `tests/cli_test.rs`,
`tests/structural_enum.rs`, and `tests/cva6_derivation.rs` carry the gates.

## The CLI surface

`ev capabilities` prints the target modules and the layers the binary carries:

```text
engine:      ev 0.1.0
targets:     riscv
formats:     xif
classifiers: structural, naive
reporters:   text, json, csv, trace
profiles:    r4, riscv-r, riscv-r4
simulation:  mock, spike
synthesis:   yosys, mock
```

The generic core (`--no-default-features`) reports `targets: (none)`,
`profiles: (none)`, and `simulation: mock`. The same data is available with
`--json`. A generic core that parses a spec naming `riscv-r` reports the
rebuild:

```text
unknown profile `riscv-r`: this build has no layout profiles
(rebuild with the default target set, or with `--features riscv`)
```

The command does not add a new record field. It reports the layer stack, and
the record contract stays what issue #66 defined.

## What did not change

No constraint type, projector, enumeration strategy, fixture count, or record
shape changed. The default target set still classifies the same spaces and
produces the same records. The generic core classifies the same spaces for
every spec that declares no profile.

## Verification

```bash
cargo test --release                                   # 163 passed, default targets
cargo test --release --no-default-features             # 152 passed, generic core
cargo test --release --no-default-features --features riscv  # 163 passed, explicit target
cargo clippy --all-targets -- -D warnings              # clean
cargo clippy --all-targets --no-default-features -- -D warnings  # clean
cargo bench --no-default-features --bench bench -- "struct_enum/ibex"  # generic core
cargo bench --bench bench -- "struct_enum/ibex"        # default targets
bash run.sh --code                                     # both states, exit 0
bash run.sh --verify                                   # default targets, exit 0
```

`run.sh --code` leaves the default target binary in `target/release/ev`, and
`run.sh --verify` checks for the profile and the backend before it runs a
fixture.

---

## References

- ev issue #70, issue #67 (the capability seam)
- `Cargo.toml`, `src/main.rs`, `src/lib.rs`, `src/profile/mod.rs`
- `tests/cli_test.rs`, `tests/structural_enum.rs`, `tests/cva6_derivation.rs`
- `run.sh`, `scripts/coverage.sh`, `benches/bench.rs`
