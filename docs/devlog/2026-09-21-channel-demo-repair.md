# Channel demo repair

Issue #53 recorded that `./run.sh --demo` could not run. This note records
what was broken, the one design choice the repair had to make, and the
evidence that the channel is now a real check rather than a green loop.

## What was broken

The script drove three surfaces that no longer exist:

| Broken | Now |
|---|---|
| `ev check --target <yaml> --json` | `ev verify --target <yaml> --format json`; `check` has not existed since the `verify` rename |
| Fixture constraints written as `- type: even` + `axis: 0` | Constraints reference fields by name: `field: "coord"` |
| `projector: { type: identity }` without a field | `identity` and `parity` take a `field`; only `sum` is field-free |
| Parsed `data['results']` from stdout | The CLI prints a Fact envelope whose `payload` is a byte vector; the `VerificationReport` JSON is inside it, so the demo decodes the envelope before reading rows |

## The design choice the repair had to make

`ev verify` reports the structurally valid subset, and the reporter derives
`failed = total - passed`. A structural constraint therefore removes a
domain point from the rows instead of marking it rejected, which breaks a
positional comparison against the assembly's per-segment golden list.

Measured, with the narrow channel expressed as `even` + structural `range`:

```bash
ev verify --target /tmp/narrow_structural.yaml --format json   # rows: 4, total: 5, passed: 2, failed: 3
```

Segment 12 is absent from the rows while the golden list expects five
entries. The fixtures therefore use runtime constraints (`even`, `ge`, `le`),
which are evaluated per combination and keep all five rows, and the script
states why.

## Golden anchors

Read from `ssccs/poc/baremetal_riscv/asm/observe_full.S`, the hand-written
RISC-V assembly that computes each segment's result:

```text
GOLDEN_SEGMENTS: 2,3,5,10,12
GOLDEN_NARROW:   2,REJECT,REJECT,10,REJECT
GOLDEN_BROAD:    2,3,5,10,12
GOLDEN_SUM3D_A:  3        GOLDEN_SUM3D_B: 6
GOLDEN_PARITY_2: 0        GOLDEN_PARITY_3: 1
```

## Evidence

| Run | Result |
|---|---|
| `SSCCS_DIR=<ssccs> bash scripts/demo-ssccs-poc.sh` | 5 / 5 channels MATCH, exit 0 |
| Same, with `GOLDEN_NARROW` altered to `2,3,REJECT,10,REJECT` | 4 / 5, the narrow channel reports MISMATCH, exit 1 |

The second row is the point: the comparison reports a disagreement, so the
channel is not a self-confirming loop. Output formatting was simplified at
the same time (plain `ok`, `MATCH`, `MISMATCH` instead of the box-drawing
characters), and an unreadable ev output is now reported as a failed channel
with the first lines of the output echoed, instead of silently comparing an
empty string.

## Running

```bash
./run.sh --demo
SSCCS_DIR=../ssccs bash scripts/demo-ssccs-poc.sh
```

The demo needs an ssccs checkout, so it stays an on-demand mode and is not
part of `run.sh --verify` or CI.

## References

- ev issue #53
- `ssccs/poc/baremetal_riscv/asm/observe_full.S`
- `scripts/demo-ssccs-poc.sh`
