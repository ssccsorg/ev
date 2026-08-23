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

Analytic conclusion: the functional contract of the decoder, of the demo top
outputs, and of the golden anchor decomposition is not expressible in the
current model. The input domain contract and the output axis bounds are the
largest expressible subsets.

A naive field-product spec over (offset, i, m, f) parses and runs under the
1B combination guard (124,813,584 combinations) but every combination passes,
so it verifies no property. The golden anchor contract therefore stays in the
domain of the Python consistency gate and the Verilator golden testbench
until the extension lands.

Sequential behavior (registered outputs, one-cycle latency) and Verilog
testbench execution are outside the ev domain, which evaluates combinational
encoding spaces. The testbench artifact is the reference for expected counts,
not a spec target.

## Milestones

Milestone 1 (this branch): fixture specs, `run.sh` assertions, CLI tests, and
this plan.

Milestone 2 (issue #46): a decode projector or an arithmetic constraint so
the decomposition becomes expressible. This is the main engine extension.

Milestone 3 (issue #46): cross-check the ev enumeration against
`syntagma/hw/rtl/golden_anchors.hex` through the golden-anchor pattern of
issues #25 and #28.

Milestone 4 (issue #46, optional): an `ev synth` design-only mode for
`tagma_decoder.v` so the generic synthesis report comes from the ev Yosys
backend.

Recommendation: ship Milestone 1 now. Milestone 2 is the prerequisite for any
functional cross-check inside ev.

## Verification results

Measured with `ev verify` on the release build, text output:

| Fixture | Total | Passed | Failed |
|---|---|---|---|
| tagma_decoder | 65,536 | 11,172 | 54,364 |
| tagma_demo_top | 11,172 | 11,172 | 0 |

---

## References

- ev issue #47: Tagma hw RTL fixture specs: feasible verification YAML specs and plan
- ev issue #46: Tagma decoder execution path verification: ev as an independent channel
- syntagma issue #48: hw: Tagma decoder RTL, FPGA verification, and standard cell synthesis report
- syntagma verification devlog: `docs/devlogs/hw/2026-08-14-verification.md`
- golden anchor pattern: ev issues #25 and #28, `ssccs/poc/baremetal_riscv/sv`
- Tagma whitepaper: https://doi.org/10.5281/zenodo.21302508
