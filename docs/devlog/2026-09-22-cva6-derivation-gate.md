# CVA6 fixture derivation gate: the fixtures against the decoder mask table

Issue #56 closes the last open link in the CVA6 claim chain. The three
decoder-derived fixtures were transcribed by hand from the coprocessor
issue-response mask table, and nothing re-derived them, so a wrong field
mapping, a missed entry, or a field the decoder pins while the fixture leaves
it free would have stayed invisible. This note records what the gate does,
what it found, and what it does not cover.

## The link that was open

`cva6/xif_ref.xif.yaml`, `cva6/xif_ref_r4.xif.yaml`, and
`cva6/xif_madd.xif.yaml` cite `core/cvxif_example/include/cvxif_instr_pkg.sv`
at commit `6544a714c`, and `instr_decoder.sv` evaluates the table as

```systemverilog
sel[i] = ((CoproInstr[i].mask & issue_req_i.instr) == CoproInstr[i].instr);
```

The other channels already had gates. Enumeration equivalence is pinned by
`tests/structural_enum.rs`, the decode projector by `tests/golden_anchor.rs`,
the generated RTL by the synthesis assertions, and the assembly golden
anchors by `./run.sh --demo`. The derivation from the hardware table had only
a header comment.

## What the gate does

`tests/cva6_derivation.rs` reads the extraction committed at
`tests/fixtures/cva6/mask_table.json`. Committing the extraction rather than
the third-party source keeps the licence question out and lets the gate run
wherever the tests run, CI included; the JSON carries the source path, the
pinned commit, and the SHA-256 of both the package and the decoder.

For each derived fixture the gate:

- places every declared field in the 32-bit word through the fixture's
  encoding layout, requiring the domain to fit the field width and no two
  fields to overlap;
- asserts the projection's precondition: every bit an entry masks must be
  covered by a declared field. An entry that masks a bit no field covers
  fails with that entry and bit named, so the projection cannot silently
  drop it;
- computes the decision axes as every field an entry masks plus every field
  the fixture's constraints reference, and enumerates that reduced space
  through the production pipeline (`expand_all` plus `evaluate_all` on a spec
  restricted to those axes), which keeps the fixture's accepted set defined
  by the same code the CLI uses;
- requires, per decision tuple, that the table accepts it exactly when the
  fixture does, and reports the differing tuples instead of a count.

The acceptance a table entry gives is
`(mask & word) == instr` for some entry, with the word packed from the
fixture's fields. Bits outside the decision axes are unconstrained in the
fixture and never masked by an entry, by the precondition, so leaving them at
zero is faithful.

## What it found

Three defects, all in the fixtures rather than in the table:

| Fixture | Finding | Fix | Valid before, after |
|---|---|---|---|
| `cva6/xif_ref.xif.yaml` | `opcode` unmodeled; every entry masked bits 0..6 | `opcode: values: [0x7B]` declared | 196,608, 196,608 |
| `cva6/xif_ref_r4.xif.yaml` | `opcode` and `rs3` unmodeled; entries masked bits 0..6 and 27..31 | `opcode: values: [0x7B]`, `rs3: values: [0]` declared | 2,560, 2,560 |
| `cva6/xif_madd.xif.yaml` | `func2` left free while the mask pins bits 26..25 to `00` | `oneof func2 [0]` added | 4,096, 1,024 |

The `xif_ref` and `xif_ref_r4` fixes add no combination: a single-valued
field multiplies the raw total by one. They matter because the table also
holds four MADD-family entries at opcodes `0x43`, `0x47`, `0x4B`, and `0x4F`.
Unmodeled opcode bits let those entries project onto the fixture's fields,
where the MADD pattern's free `funct7` high bits would have added
`(funct3, funct7)` pairs at `(0, 1)`, `(0, 2)`, and `(0, 3)` that the fixture
does not accept. Declaring the opcode space is what makes the table's
restriction to the fixture equal to the fixture's accepted set.

The `xif_madd` finding is the one the gate exists for: `func2` is the R4
`fmt` field, the mask pins it, and the fixture accepted 3,072 combinations
the coprocessor decoder rejects. `run.sh` now carries a `_verify_check` for
that fixture, so the corrected count is pinned instead of printed.

## Measured result

| Fixture | Decision axes | Table tuples | Fixture tuples | Constrained pairs |
|---|---|---:|---:|---|
| `cva6/xif_ref.xif.yaml` | funct3, funct7, opcode | 6 | 6 | 6 `(funct3, funct7)` |
| `cva6/xif_ref_r4.xif.yaml` | func2, funct3, opcode, rs3 | 5 | 5 | 5 `(funct3, func2)` |
| `cva6/xif_madd.xif.yaml` | func2, funct3, opcode, rs3 | 16 | 16 | 1 `(funct3, func2)` |

The `xif_ref` pairs are `(0,0)`, `(1,0)`, `(1,1)`, `(1,2)`, `(1,3)`, `(1,4)`,
asserted by `xif_ref_reports_the_six_decoder_pairs` against a literal set
rather than against the table it was computed from.

Sensitivity, checked by reverting each fix in turn: unmodeled `opcode` in
`xif_ref` and `xif_ref_r4` fails naming `NOP` and bit 0; a free `func2` in
`xif_madd` fails with 48 of the 512 decision tuples listed, every one of them
a `func2 != 0` combination the decoder rejects.

## Source channel

When a checkout is at hand (`CVA6_DIR`, default `../cva6`),
`source_mask_table_matches_the_checkout` requires the checkout to be at the
pinned commit, both file digests to match the committed copy, and a fresh
extraction to equal it. A commit mismatch fails naming both revisions. An
absent checkout is reported as unavailable, never passed over silently;
`run.sh --verify` prints which of the two states applies.

`EV_UPDATE_MASK_TABLE=1 cargo test --release --test cva6_derivation
update_mask_table_when_requested` rewrites the committed extraction from a
checkout, so a pin move is a one-command change whose diff shows the table.

## Coverage boundary

- CI checks out `ev` alone, so CI runs the derivation channel against the
  committed extraction and the source channel does not run there. The
  committed extraction is therefore the artifact CI trusts, and the source
  channel is what keeps it honest locally.
- The extraction is committed, not generated at build time. A change to the
  table without regenerating the JSON is caught by the source channel only
  where a checkout exists.
- The gate covers the three fixtures whose headers cite the table.
  `cva6/xif_mac.xif.yaml` is a hand-written accelerator model, and
  `cva6/xif_encoding.xif.yaml` models the DV class
  (`cvxif_custom_instr.sv`), which deliberately generates encodings the
  reference decoder rejects.
- `CVA6_DIR` pointing at a directory that is not a git work tree is reported
  as unavailable rather than failed, because the revision cannot be read;
  the file digests would still be checked had the tree resolved.

## Running

```bash
cargo test --release --test cva6_derivation
CVA6_DIR=/path/to/cva6 cargo test --release --test cva6_derivation
EV_UPDATE_MASK_TABLE=1 cargo test --release --test cva6_derivation update_mask_table_when_requested
./run.sh --verify
```

## References

- ev issue #56, issue #55 (the projector generalization this follows), issue
  #19 (the original fixture derivation)
- `cva6/core/cvxif_example/include/cvxif_instr_pkg.sv` at
  `6544a714c20193b94612f64a6aa43d9299feda49`,
  SHA-256 `a498af9cdfc4ef0238f2cfbf5d24579ee5e9126443917675f5d5ccb8e022bee3`
- `cva6/core/cvxif_example/instr_decoder.sv` at the same commit,
  SHA-256 `6116f241a09a0494600319f33f41ea61e545c92e4ad729ced4e5c9c39337f3ca`
