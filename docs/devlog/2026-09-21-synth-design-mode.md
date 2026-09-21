# Design-only synthesis, and the metrics that were never parsed

Issue #46 milestone 4 asked for `ev synth --design`, so the Tagma decoder's
generic synthesis report could come from the ev Yosys backend instead of the
mirrored script in syntagma. Delivering it surfaced two defects in that
backend, which this note records alongside the CLI change.

## CLI change

```bash
ev synth --target <yaml> [--json]          # generate RTL from a spec, then synthesize
ev synth --design <rtl> [--top <module>]   # synthesize an RTL file directly
```

- `--target` and `--design` are mutually exclusive, and one is required.
- `--top` applies to `--design` only and defaults to the design file stem.
  A stem that does not name a module leaves Yosys to fail on
  `hierarchy -top`, which is louder than guessing.
- `--design` requires a readable file; a missing path fails before Yosys runs,
  naming the path.
- `RunSynthesis` already took an RTL path and a top module, so the trait
  needed no change. `SynthInput` in `main.rs` carries the two shapes.

## Defects found and fixed

Checking the acceptance criterion (the same metrics as syntagma's generic
flow) showed `gate count: None` on a real Yosys run. Two independent causes:

1. The script redirected with `stat -json > {file}`. Inside `yosys -p`, that
   redirection does not create the file. Verified by running the mirrored
   syntagma invocation: with `>` no report is written, with
   `tee -o {file} stat -json` it is. Every real-Yosys run therefore had no
   report to parse, so `gate_count` and `cell_area` were `None`. The spec
   path (`--target`) was affected in the same way, and only the status was
   ever asserted, so it went unnoticed.
2. The cell-type breakdown was read from `modules[<top>].cells`, which Yosys
   does not emit. The report carries `modules[\<top>].num_cells_by_type`,
   keyed by the escaped module name.

Both are fixed: the script uses `tee -o`, and a `parse_stat` helper reads the
aggregate from `design`, falls back to the module entry, and takes the cell
types from `num_cells_by_type`. `cell_area` stays `None` without `-liberty`,
which the generic flow does not use.

## Measurements

| Input | ev `gate count` | Reference |
|---|---:|---|
| `syntagma/hw/rtl/tagma_decoder.v` (`--design`, `--top tagma_decoder`) | 478 | 478 in syntagma's committed `hw/synth/yosys/reports/generic_stat.txt`, and 478 from a mirrored run of the same invocation |
| `tests/fixtures/rtl/decode_demo.v` (`--design`) | 16 | cell types: 5 `$_AND_`, 1 `$_NAND_`, 7 `$_XNOR_`, 3 `$_XOR_` |
| `tests/fixtures/common/all_pass.xif.yaml` (`--target`) | 28 | the generated simple ALU |

Yosys 0.65, single machine.

## Guards added

- `run.sh --verify` synthesizes `tests/fixtures/rtl/decode_demo.v` through
  `--design` and fails when the status is not ok or the gate count is absent.
  The null-count branch is the regression guard for defect 1.
- The `--target --json` check now also requires a populated `gate_count`,
  which is what a consumer reads. Sensitivity checked by running the
  assertion against a deliberately null count: it fails, and passes with a
  populated one.
- Tests: four lib tests over `parse_stat` (aggregate, module fallback, absent
  report, absent module) and seven CLI tests for the design path (stem
  default, `--top` override, JSON envelope naming the design source, missing
  file, no input, `--top` with `--target`, `--target` with `--design`).

## References

- ev issue #46 (milestone 4), issue #52 (the golden anchor gate on the same branch)
- `src/synth/backends/yosys.rs`, `src/main.rs`, `run.sh`
- syntagma `hw/synth/yosys/synth_generic.ys`, `reports/generic_stat.txt`
