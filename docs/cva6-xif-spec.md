# CVA6 CV-X-IF: Specification, Validation, and Cross-Reference

## Source

- CVA6 RTL: `cva6/core/cvxif_instr_pkg.sv`, `cva6/core/cvxif_fu.sv`
- CVA6 Verification Suite: `cva6/verif/env/corev-dv/custom/cvxif_custom_instr.sv`
- XIF Spec: `core-v-xif/docs/xif_specification.adoc`
- ev Fixture: `tests/fixtures/cva6_xif_ref.xif.yaml`

## Architecture

CVA6 offloads instructions to the XIF coprocessor when the core decoder marks
them as **illegal**. Custom opcodes (`custom-0` through `custom-3`, opcodes
`0x0B`, `0x2B`, `0x5B`, `0x7B`) are always illegal and therefore always
offloadable. FP opcodes (MADD/MSUB/NMSUB/NMADD) are offloadable only when
the FP unit is disabled.

## Instruction Encoding (verification suite mapping)

All custom-3 instructions use opcode `0x7B` (7'b1111011).

| funct3 | funct7 | Instruction | rs1 | rs2 | rd | register_read |
|--------|--------|-------------|-----|-----|-----|--------------|
| 000    | 2 (0b0000010) | CUS_U_ADD | rs1 | rs2 | rd | [rs1, rs2] |
| 000    | 6 (0b0000110) | CUS_S_ADD | rs1 | rs2 | rd | [rs1, rs2] |
| 000    | 8 (0b0001000) | CUS_ADD_MULTI | rs1 | rs2 | rd | [rs1, rs2] |
| 000    | 32 (0b0100000) | CUS_ADD_RS3 | rs1 | rs2 | rd(rs3) | [rs1, rs2, rs3] |
| 001    | 0 (0b0000000) | CUS_ADD | rs1 | rs2 | rd | [rs1, rs2] |
| 010    | 96 (0b1100000) | CUS_EXC | rs1 | — | — | [rs1] |
| 011..111 | any | illegal | — | — | — | — |

Note: CUS_NOP uses the same encoding as CUS_ADD (funct3=001, funct7=0) with
rd=0, rs1=0, rs2=0. It is distinguished at the decode level by the coprocessor,
not by instruction bits.

CUS_ADD_RS3 uses func2=2'b01 (bits [26:25]) within funct3=000, funct7=32.

R4-type opcodes (MADD/MSUB/NMSUB/NMADD) are not modeled in the current ev
fixture because their offloadability depends on FP configuration state.

## ev Fixture: cva6_xif_ref.xif.yaml

| Field | Domain | Rationale |
|-------|--------|-----------|
| funct3 | [0, 7] | Full 3-bit RISC-V funct3 field |
| funct7 | [0, 127] | Lower 7 bits of funct7 (bits [31:25]) |
| rs1 | [0, 31] | Full RV32 GPR |
| rs2 | [0, 31] | Full RV32 GPR |
| rd | [0, 31] | Full RV32 GPR |

## Constraints

```text
oneof:  funct3 in {0, 1, 2}
cross:  funct3=0 -> funct7 in {2, 6, 8, 32}
        funct3=1 -> funct7 = 0
        funct3=2 -> funct7 = 96
```

## Verification Results

| Metric | Value |
|--------|-------|
| Raw combinations | 33,554,432 (8 x 128 x 32 x 32 x 32) |
| Valid (passed) | 196,608 |
| Illegal (failed) | 33,357,824 |
| Execution time | 32 seconds (Apple M1 Max, single core) |
| Spike cross-validation | 196,608 / 196,608 passed (100%) |

Valid breakdown by funct3:

- funct3=0: 4 x 32 x 32 x 32 = 131,072 (U_ADD, S_ADD, ADD_MULTI, ADD_RS3)
- funct3=1: 1 x 32 x 32 x 32 = 32,768 (CUS_ADD)
- funct3=2: 1 x 32 x 32 x 32 = 32,768 (CUS_EXC)

## Comparison: Exhaustive vs Random (riscv-dv)

| Aspect | riscv-dv (Random) | ExaVerif (Exhaustive) |
|--------|-------------------|----------------------|
| Unique encodings evaluated | ~10^6 (sampled) | 33.5 x 10^6 (complete) |
| Coverage guarantee | Statistical | Exhaustive (100%) |
| Time to result | Minutes to hours | 32 seconds |
| Setup complexity | UVM agent + config files | Single YAML |
| Cross-validation | RTL simulation only | Static + Spike RISC-V sim |
| Register distribution | Weighted random | Uniform exhaustive |

## Coverage Gaps

### What ev covers that riscv-dv misses

1. **Full funct3/funct7 enumeration**: riscv-dv may miss specific encoding
   combinations depending on random seed. ev covers every combination.
2. **Deterministic reproducibility**: Same input always produces same output.
   No seed management required.
3. **Explicit constraint specification**: Encoding rules are documented in
   machine-readable YAML, not embedded in SV code.

### What riscv-dv covers that ev misses

1. **CUS_EXC weighted distribution**: rs1 distribution is biased toward
   edge values (0-9, 10, 14, etc.). ev treats all register values uniformly.
2. **CUS_NOP register suppression**: NOP instruction disables rd/rs1/rs2.
   ev has no mechanism for conditional field activation.
3. **CUS_ADD_RS3 func2 encoding**: Uses R4-type func2 field. Not yet modeled.
4. **Pipeline hazards**: rs1==rd write-after-read scenarios exercised by
   random instruction sequences.

## Future Work

| Feature | Priority | Status |
|---------|----------|--------|
| CUS_ADD_RS3 with func2 (R4 opcode) | Medium | Not started |
| Conditional field activation (NOP) | High | Design phase |
| Weighted distribution constraints | Low | Not started |
| Spike integration in CI pipeline | High | Needs Docker image |
