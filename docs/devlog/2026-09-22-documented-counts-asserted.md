# The documented counts are now asserted

Issue #59 closes the second gap the provenance matrix recorded: four fixtures
had counts in the README and `AGENTS.md` and no assertion in `run.sh`. The gate
covers the derivation, and the count is what catches a change to the space
itself, which is the failure #56 found when `xif_madd` accepted 3,072
combinations the decoder rejects and nothing failed.

## What is asserted now

| Fixture | Raw | Passed | Failed | Asserted in |
|---|---:|---:|---:|---|
| `cva6/xif_encoding.xif.yaml` | 8,192 | 48 | 8,144 | `verify_large_fixtures` |
| `ibex/csr_access.xif.yaml` | 49,152 | 49,152 | 0 | `verify_large_fixtures` |
| `common/all_pass.xif.yaml` | 1,024 | 1,024 | 0 | `verify_fixtures` |
| `common/sample.xif.yaml` | 96 | 12 | 84 | `verify_fixtures` |

`_verify_check` compares both counts and sets `VERIFY_FAILED`, so a drift fails
`run.sh --verify` and the default pipeline rather than showing up as a different
number in the log.

## What the work corrected

The issue said none of the four had a count assertion, and that was too strong
for one of them: `tests/cli_test.rs` already asserted `failed: 84` for
`sample`. It now asserts the passing count as well, and the README row no longer
claims the fixture is unasserted.

Two fixtures had no header comment at all, so a reader met `target:` and
nothing else. Both now state their space, the arithmetic behind it, and why they
exist.

That header work surfaced a third defect, in `common/sample.xif.yaml`:

```yaml
target: CVA6_XIF_Accelerator
```

The space is synthetic, its fields (`operand_a`, `hazard_bypass`,
`pipeline_depth`) model no hardware, and the README's provenance table listed
its source as none. The target name claimed a provenance the fixture does not
have, which is the same class of defect as #55 (a sample's name and constants
inside the engine) one layer up, in a fixture. Renamed to `accelerator_demo`,
with the header stating that the target names the shape of the space and not a
provenance. Nothing asserted the old name: it appeared in the fixture alone.

## Measured result

`bash run.sh --verify` runs the four checks and reports each:

```
=== all-pass fixture ===     target: simple_alu        passed: 1024, failed: 0 — ok
=== mixed fixture ===        target: accelerator_demo  passed: 12, failed: 84 — ok
=== cva6 xif encoding ===    target: cva6_xif_encoding passed: 48, failed: 8144 — ok
=== ibex csr access ===      target: ibex_csr_access   passed: 49152, failed: 0 — ok
```

Sensitivity, by changing the `xif_encoding` expectation to 47 / 8,145:

```
=== cva6 xif encoding ===
  FAILED: expected 47 passed, got 48
  Some fixture assertions FAILED!
exit=1
```

## Verification

```bash
cargo test --release          # 130 passed, 0 ignored
cargo fmt --all --check       # clean
cargo clippy --all-targets    # clean under -D warnings
bash run.sh --verify          # Verification passed, all four new checks ok
```

## References

- ev issue #59, issue #57 (the provenance matrix that found the gap), issue #56
  (the derivation gate), issue #51 (the earlier count-pinning pass)
- `run.sh` (`verify_fixtures`, `verify_large_fixtures`), `tests/fixtures/common/`
