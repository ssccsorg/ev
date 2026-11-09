# Design-basis case space

An example of `ev verify` on a design-basis matrix. The space is declared, the
combination rules are declared, and the record states how many points the rules
reject and which spec produced the result.

The field values and the rules are synthetic. They model no code, component, or
plant. What is real is the shape: a full matrix of load cases against regions,
materials, and methods, narrowed by rules that state which combinations the
design basis admits.

## The space

| Field | Values | Meaning |
|---|---|---|
| `revision` | 0..2 | design revision |
| `load_case` | 0..11 | load combination entries |
| `region` | 0..4 | nozzle, shell, head, support, flange |
| `material` | 0..2 | SA-508 Gr.3 Cl.1, SA-533 Type B Cl.1, 316L |
| `method` | 0..1 | design by formula, design by analysis |

Raw space: 3 x 12 x 5 x 3 x 2 = 1,080 combinations.

### Declared load-case entries

These are the entries this space declares. They are not a code table; a real
instance replaces them with the ones its plant design specification states.

| Index | Entry | Declared combination |
|---|---|---|
| 0 | design | `P_des + D` |
| 1 | Level A | `P_o + D + T` |
| 2 | Level B | `P_o + D + T + OBE` |
| 3 | Level C | `P_o + D + T + SSE` |
| 4 | Level D | `P_o + D + T + SSE faulted` |
| 5 | hydrostatic test | `1.25 P_des` |
| 6 | pneumatic test | `1.10 P_des` |
| 7 | Level A, fatigue | `P_o + D + T + OBE`, fatigue evaluation |
| 8 | Level B, relief valve | `P_o + D + T + OBE + SRV` |
| 9 | Level C, pipe break | `P_o + D + T + SSE + pipe break` |
| 10 | Level D, pipe break | `P_o + D + T + SSE faulted + pipe break` |
| 11 | design with OBE | `P_des + D + OBE` |

Two rules narrow it, and both are encoded into the enumeration, so a rejected
combination is never emitted as a point:

| Rule | Statement |
|---|---|
| `region` to `material` | a support is 316L only; a nozzle and a flange take SA-508 or 316L; a shell and a head take SA-508 or SA-533 |
| `load_case` to `method` | the hydrostatic and pneumatic test entries require design by analysis; every other entry admits both |

Admissible: 3 x (2 + 2 + 2 + 1 + 2) x (10 x 2 + 2 x 1) = 594. Rejected: 486.

## Run

```bash
./run.sh              # summary, exit status, and the record identity
./run.sh --json       # the Fact envelope
./run.sh --record     # the stable payload
./run.sh --reproduce  # payload identical across runs, envelope stamp moves
```

`ev verify` exits 1 when the declared space contains a rejected combination.
That is a verdict on the space, so `run.sh` prints the status as a value.

## Expected output

For the spec in this directory:

```text
target: smr_nsss_case_space
engine: ev/structural 0.1.0
claim:  exhaustive
total:  1080
passed: 594
failed: 486

exit:   1 (1 means the space contains a rejected combination)

record
  spec_hash: dd108e8924cb308f5af9e0eca930358fd3b1487f42fa5127d026100a3f3a910a
  points:    594
```

`./run.sh --reproduce` prints the payload hash twice and the envelope stamp for
each run:

```text
payload hash, run 1: 8a443eded22d93f948857c8b35fca11bcd93864a3325eb613f4e1754995cde34
payload hash, run 2: 8a443eded22d93f948857c8b35fca11bcd93864a3325eb613f4e1754995cde34
envelope stamp, run 1: 2026-11-09T10:18:54.640526+00:00
envelope stamp, run 2: 2026-11-09T10:18:54.657137+00:00
payloads identical
```

Both hashes are tied to the spec: a change to a domain, a rule, or the
projector changes `spec_hash` and therefore the payload hash. The stamps differ
by construction.

## Scale

`study.xif.yaml` applies the same two rules to a design-of-experiments space:
2,000 geometry variants, 24 load cases, 5 regions, 3 materials, 2 methods.

| Spec | Declared | Admissible | Wall | Peak RSS |
|---|---|---|---|---|
| `space.xif.yaml` | 1,080 | 594 | 0.00 s | 4 MB |
| `study.xif.yaml` | 1,440,000 | 828,000 | 0.13 s | 188 MB |

`total` is the size of the declared product, computed from the domain
cardinalities without enumerating it. `passed` is the number of points the
engine emitted and classified. Memory follows the emitted count at roughly 230
bytes a point, so a space with tens of millions of admissible points does not
fit this pipeline; the streaming path is a separate, open item.

```bash
SPEC=study.xif.yaml ./run.sh
```

## Recording the demo

`asciinema` records a terminal session as text with real timing, so the
recording replays what was typed and the commands stay copyable. It is not
installed on this machine.

```bash
brew install asciinema
asciinema rec --title "smr_cases" -c "./run.sh && ./run.sh --reproduce" demo.cast
asciinema play demo.cast     # local replay
asciinema upload demo.cast   # prints a URL
```

Check `asciinema rec --help` for the flag names in the installed version. An
uploaded cast is readable by anyone with the link, so inspect it with
`asciinema cat demo.cast` before sharing.

## What the record carries

| Field | Meaning |
|---|---|
| `spec_hash` | identity of the spec; a change to a domain, a rule, or the projector changes it |
| `engine`, `claim` | which engine classified, and what it claims (`exhaustive`) |
| `total`, `passed`, `failed` | declared points, admissible points, rejected points |
| `point_id` | identity of a coordinate, verdict excluded |
| `id` | identity of the observation, verdict and projection included |

The payload is identical across runs. The envelope adds a timestamp and an
origin, which is why two `--json` runs differ and two `--record` runs do not.

## What this does not claim

- No allowable is evaluated. The verdict is admissibility per the declared
  rules, not a stress or temperature check.
- Nothing here replaces an FEM solve. The admissible set is the case list.
- Whether each admissible case was run and met its allowables is a separate
  input. A second spec over assessment results would carry it.

## Using it on a real case list

Replace the synthetic domains with the plant's declared load combinations,
regions, materials, and methods, and replace the two rules with the ones the
design basis states. The question stays the same, and `spec_hash` becomes the
identity of that case list: two studies with the same hash declared the same
cases.
