# Repo hygiene: docs retirement, fixture relabel, and pinned fixture counts

Issue #51 clears the artifacts left behind by three earlier moves: the public
documentation of record moving to the ssccs corpus, the structural pipeline
becoming the CLI default, and the Ibex fixture work landing as the
`rv32imcb*` pair. This note records what was removed, what was relabelled,
and what is now pinned.

## The public docs pipeline was retired

The public documentation of record for ev is now `ssccs/docs/projects/ev/`
(`index.qmd`, `cva6.qmd`, `ibex.qmd`, `partnerships/exa-aws.qmd`), published
as https://docs.ssccs.org/projects/ev/, and the Claude-readable artifact at
`s3://ssccs-nexus-af/ev/docs` comes from that corpus.

The ev repository kept the machinery of its own retired site, and the
machinery no longer resolved:

| Removed | Why |
|---|---|
| `.github/workflows/build-docs.yml` | Published `ev-docs` to Cloudflare Pages, synced artifacts to R2, and dispatched the nexus build, all for a site with no pages left in `docs/` |
| `docs/_quarto-website.yml` | Declared `site-url: https://ev.ssccs.org` and a sidebar whose first entry, `index.qmd`, had moved to the corpus |
| `docs/_quarto.yml`, `docs/build.yml` | Site project and target configuration, including a pre-build step against a sibling `../poc/...` checkout that the CI workspace never provided |
| `docs/build.sh`, `docs/_quarto_pre-render.py` | `sdb build` invocation and metadata pre-render for the retired site |
| `docs/_include/` | Partials for the retired site; the corpus pages resolve their own `../../_include/` |
| `docs/arch/generate.py` | Generated a site page that consumed the removed partials |

`docs/README.md` now states the scope: private notes only, public
documentation belongs to the corpus. `docs/devlog/` is unchanged.

## The synthetic fixture no longer claims to be Ibex hardware

`tests/fixtures/ibex/alu_ext.xif.yaml` described itself as a "hypothetical
spec" based on Ibex, and issue #36 recorded that it corresponded to no actual
Ibex configuration. Its real role is the fixture where `oneof`, `cross`, and
`enable_mask` are active together with an `identity` projector, which is what
`tests/structural_enum.rs` pins.

Renamed to `tests/fixtures/common/enable_mask_demo.xif.yaml`, target
`enable_mask_demo`, with a header that states the coverage role and points at
the real Ibex decoder fixtures. References updated in `run.sh`,
`tests/structural_enum.rs`, `tests/cli_test.rs`, and the README fixture
table. The dated devlog entry from 2026-08-23 that mentions the old name is a
record of that day and was left as is.

## Fixture counts are now asserted, not printed

`run.sh` ran `cva6/xif_mac.xif.yaml` and the enable_mask fixture with `_timed`
only, so their counts were visible without being pinned. Both now carry
`_verify_check` assertions:

| Fixture | Passed | Failed |
|---|---:|---:|
| `cva6/xif_mac.xif.yaml` | 28,672 | 4,096 |
| `common/enable_mask_demo.xif.yaml` | 4,096 | 520,192 |

## AGENTS.md

The handoff described the May 2026 tree (commit `547e2a9`, 9 constraint
types, 3 projectors, 69 tests, flat `src/*.rs`, "Spike backend not started").
It now matches the current module layout, the 13 constraints and 4
projectors with their structural and runtime split, the structured CLI default, the coverage gate, the test and fixture inventory, the backend
environment variables, the docs scope, and the open work (#46 milestone 4,
#44, #18, FIH alignment deferred).

## Verification

```bash
./run.sh --code      # fmt, clippy, build, test
./run.sh --verify    # fixtures, golden anchors, synthesis, simulation
```

## Single-branch consolidation

The hygiene work and the golden anchor gate (issue #52) share one branch,
`52-tagma-golden-anchor-gate`. Two branches imposed a merge-order constraint
on the documents they shared (the README fixture table, test tree, and
Validation Results row, plus `AGENTS.md`) with no offsetting benefit.
`AGENTS.md`, the README test tree, and the test count therefore describe the
branch as it stands: 118 tests, including `tests/golden_anchor.rs`.

## Consistency fixes

Four consistency defects surfaced while aligning the documents with the code
and were fixed in the same pass:

- The `enable_mask_demo` header claimed that the CVA6 and Ibex fixtures do
  not combine `oneof`, `cross`, and `enable_mask`. `cva6/xif_encoding.xif.yaml`
  does. The header now states the real distinction: the parity invariant is
  asserted on the distinct passing set here, while `xif_encoding` is asserted
  by counts. The adjacent comment in `tests/structural_enum.rs` was corrected
  for the same reason.
- `tests/cli_test.rs` still named its CSR test `verify_rv32i_csr_access_fixture`,
  which matched neither the old target nor the new one. Renamed to
  `verify_ibex_csr_access_fixture`.
- The README architecture tree lagged the layout: `fact decode` was missing
  from the CLI line, `EncodingLayout` and `FieldBitMapping` from `spec/`, and
  `fih.rs` was described as content-addressed although `Fact` carries an
  opaque `payload` with no hash. All three now match the code, and `AGENTS.md`
  records that the two trees are kept in step.
- `.gitignore` kept the Quarto and docs-build sections (`_cached`, `_site`,
  `_pages`, `_jupyter_cache`, `**/*.c2pa`, `**/*_llms`) for a pipeline that no
  longer exists in this repository. Removed.

## References

- ev issue #51, issue #36, issue #33 task 3
- `ssccs/docs/projects/ev/` (public docs of record)
