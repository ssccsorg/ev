# Tagma hardware RTL fixture specs: feasibility and plan

The syntagma hardware track (syntagma issue #48) delivered four RTL artifacts
in `syntagma/hw/rtl/`. This devlog records which of the four can be expressed
as ev verification specs with the current constraint set, which cannot, and
the milestones that close the gap. The work follows ev issue #46 (Tagma
decoder execution path verification), which scopes the engine extension this
plan needs, and delivers the fixture slice under ev issue #47.

## The four artifacts

| Artifact | Role | Feasible as ev spec today |
|---|---|---|
| `tagma_decoder.v` | Combinational 3-axis decoder, code in [0xAC00, 0xD7A3] to (i, m, f) | Input domain contract |
| `tagma_demo_top.v` | FPGA demo top, registered decoder outputs plus valid LED | Output axis space bounds |
| `tagma_decoder_tb.v` | Exhaustive Verilator testbench over 11,172 code points | Not expressible |
| `golden_anchors.hex` | 11,172 packed 29-bit reference vectors | Format documentable, decomposition not expressible |

## Delivered fixtures

`tests/fixtures/tagma/tagma_decoder.xif.yaml` models the valid input domain:
field `code` over all 65,536 16-bit values, constrained by `ge 0xAC00` and
`le 0xD7A3`. The expected result is 11,172 passed and 54,364 failed, which
pins the boundary correction recorded in the syntagma verification devlog:
the last valid syllable is U+D7A3, not U+D7AF, because 0xAC00 + 11171 = 0xD7A3.

`tests/fixtures/tagma/tagma_demo_top.xif.yaml` models the output axis space:
fields `i` in [0, 18], `m` in [0, 20], `f` in [0, 27], 11,172 combinations,
all valid. The demo top validity LED uses the same predicate as the decoder
domain, and its registered outputs are the same decoder function, so this
fixture documents the codomain rather than adding a new predicate.

Both fixtures are wired into `run.sh` fixture assertions,
`tests/cli_test.rs`, and `tests/tagma_fixture.rs`.

## Feasibility boundary

The decoder decomposition `i = offset / 588`, `m = (offset % 588) / 28`,
`f = offset % 28` requires integer division and modulo by constants. The 13
constraint types (range, even, eq, neq, lt, gt, le, ge, oneof, cross, bitmask,
enable_mask, enable_set) express relations between field values and constants,
not arithmetic compositions of fields.

Analytic conclusion, as of milestone 1: the functional contract of the
decoder, of the demo top outputs, and of the golden anchor decomposition was
not expressible in the model. The input domain contract and the output axis
bounds were the largest expressible subsets.

Milestone 2 closes the projection side of that gap: the `tagma_decode`
projector computes the decomposition and packs it into the golden-anchor
layout offset[28:15] i[14:10] m[9:5] f[4:0]. The decomposition is now a
projection, not a constraint: pass/fail still expresses only the domain, and
the axes are verified by comparing projections to the anchor contract.

A naive field-product spec over (offset, i, m, f) parses and runs under the
1B combination guard (124,813,584 combinations) but every combination passes,
so it verifies no property.

Sequential behavior (registered outputs, one-cycle latency) and Verilog
testbench execution are outside the ev domain, which evaluates combinational
encoding spaces. The testbench artifact is the reference for expected counts,
not a spec target.

## Coverage gate

`scripts/coverage.sh`, wired into `run.sh --coverage` and `make coverage`,
runs the instrumented suite with cargo-llvm-cov, then exercises the Spike,
Yosys, and simulation backends with the instrumented binary (local tools or
the ev Docker image), then merges all profraw data and fails the build when
lines or regions fall below 80%. Measured on the committed suite: 83.35%
lines and 82.34% regions across the full crate, including the backends. The
gate runs in the CI coverage job.

## Milestones

Milestone 1 (this branch, delivered): fixture specs, `run.sh` assertions,
CLI tests, lib-level fixture tests, the CI coverage gate, and this plan.

Milestone 2 (issue #46, delivered on this branch): the `tagma_decode`
projector, registered in `ProjectorRegistry` with the packed golden-anchor
output, plus the `sv_projector` arm so generated SystemVerilog preserves the
layout. Spot and full-domain tests pin the packing (projection of code k
equals line k of the anchor file).

Milestone 3 (issue #46, next): cross-check the ev projections against
`syntagma/hw/rtl/golden_anchors.hex` line by line. The projector already
emits the anchor layout, so this is a direct comparison, following the
golden-anchor pattern of issues #25 and #28.

Milestone 4 (issue #46, optional): an `ev synth` design-only mode for
`tagma_decoder.v` so the generic synthesis report comes from the ev Yosys
backend.

## Verification results

Measured with `ev verify` on the release build, text output:

| Fixture | Total | Passed | Failed |
|---|---|---|---|
| tagma_decoder | 65,536 | 11,172 | 54,364 |
| tagma_demo_top | 11,172 | 11,172 | 0 |

The tagma_decode projections match the golden-anchor contract at the
boundaries and over the full domain: code 0xAC00 projects to 0x00000000,
0xAC01 to 0x00008001, 0xAC02 to 0x00010002, and 0xD7A3 to the packed last
syllable (offset 11171, i 18, m 20, f 27).

---

## References

- ev issue #47: Tagma hw RTL fixture specs: feasible verification YAML specs and plan
- ev issue #46: Tagma decoder execution path verification: ev as an independent channel
- syntagma issue #48: hw: Tagma decoder RTL, FPGA verification, and standard cell synthesis report
- syntagma verification devlog: `docs/devlogs/hw/2026-08-14-verification.md`
- golden anchor pattern: ev issues #25 and #28, `ssccs/poc/baremetal_riscv/sv`
- Tagma whitepaper: https://doi.org/10.5281/zenodo.21302508
