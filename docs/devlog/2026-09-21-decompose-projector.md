# The tagma_decode projector becomes a general decompose projector

Issue #55 removed the last target-specific code from the engine. The
projector that packs a field's mixed-radix decomposition carried a sample's
name and constants inside `ev`, which read as a verifier for one target in an
otherwise target-agnostic tool. This note records the replacement shape, the
one behaviour change, and what now guards it.

## Before

| Location | Content |
|---|---|
| `src/spec/mod.rs` | `tagma_decode` variant, default base `0xAC00` |
| `src/verify/registry.rs` | registration, `TAGMA_STRIDE_INIT = 588`, `TAGMA_STRIDE_MED = 28`, `TagmaDecodeEval` |
| `src/synth/mod.rs` | the SystemVerilog arm, with the expression written out for that layout |

Every other core enters through fixtures: `cva6`, `ibex`, `rv32imcb`, and
`cvxif` appeared in `src/` only in two `spike.rs` comments.

## After

```yaml
projector:
  type: decompose
  field: "code"
  base: 0xAC00
  offset_shift: 15
  axes:
    - { radix: 28, width: 5, shift: 0 }   # f = offset % 28
    - { radix: 21, width: 5, shift: 5 }   # m = (offset / 28) % 21
    - { width: 5, shift: 10 }             # i = offset / 588
```

The field value is reduced by `base`, then split along `axes` from the least
significant axis: an axis with a `radix` takes the next mixed-radix digit, and
an axis without one takes the residual. The reduced offset and the digits are
packed at their shifts. The syntagma anchor layout is that configuration, and
the tagma fixture now states it in YAML while the engine states nothing.

Parameters are validated when the spec is parsed, so a malformed spec fails
before evaluation with a message that names the axis: an empty axis list, a
radix below 2, a width that cannot hold the radix, overlapping axes, an offset
shift that overlaps the axes, or a field wider than 63 bits.

The SystemVerilog arm now derives the expression from the same parameters. For
the anchor layout it emits `((code - 44032) << 15) | ((code - 44032) % 28) |
(((code - 44032) / 28) % 21) | (((code - 44032) / 588) << 10)`, which is the
same packing expressed through the divisor chain instead of the previous
`% 588` form.

## Behaviour change

The old evaluator also enforced the Tagma domain (i < 19, m < 21, f < 28)
inside the projector, duplicating what the fixture's `ge`/`le` constraints
already said. The general projector bounds each axis by its `width` only, so a
code point above U+D7A3 now projects instead of returning nothing, and the
domain boundary stays where the constraints put it. The fixture's pinned counts
are unchanged: 11,172 valid of 65,536, because the boundary was never the
projector's job.

## Guards

- Five lib tests over the general evaluator: the anchor layout reproduced from
  its parameters, packing without an offset term, a value below the base, the
  width bound as the only axis bound, and the validation rejections.
- `verify_malformed_decompose_projector_exits_nonzero` covers the parse-time
  path through the CLI, and `tests/fixtures/common/malformed_decompose.xif.yaml`
  is the malformed input.
- `tests/tagma_fixture.rs` keeps the spot values, the full-domain projection
  check, and the SystemVerilog expression assertion, now against the derived
  form.
- The golden anchor gate from issue #52 is unaffected: it reads the fixture,
  and the projections it compares are unchanged.

The earlier devlogs keep the old name, since they record the day it was
introduced.

## References

- ev issue #55, issue #46 (the projector's origin), issue #52 (the gate)
- `tests/fixtures/tagma/tagma_decoder.xif.yaml`, `src/spec/mod.rs`, `src/verify/registry.rs`
