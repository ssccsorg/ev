# CVA6 XIF Spec Space — ev Verification Fixture Design

Based on analysis of `core-v-xif/` specification and `cva6/core/` source.

## Architecture Overview

CVA6 offloads instructions to XIF coprocessor when the core decoder marks them as **illegal**. Custom opcodes (`custom-0` through `custom-3`, opcodes `0x0B`, `0x2B`, `0x5B`, `0x7B`) are **always** illegal → always offloadable. FP opcodes (MADD/MSUB/NMSUB/NMADD) are offloadable only when FP is disabled.

## Example Coprocessor Encoding Space

The reference coprocessor (`cvxif_instr_pkg.sv`) defines 10 instructions within `custom-3` (opcode `0x7B`) and R4 opcodes.

### Custom-3 opcode (`0x7B`)

| funct3 | funct7[26:25] | funct7[4] | Instruction | rs1 | rs2 | rs3 | Writeback | register_read |
|--------|---------------|-----------|-------------|-----|-----|-----|-----------|--------------|
| 000    | 00            | 0         | NOP         | -   | -   | -   | No        | []           |
| 001    | 00            | 0         | ADD         | rs1 | rs2 | -   | rd        | [rs1, rs2]   |
| 001    | 01            | 0         | DOUBLE_RS1  | rs1 | -   | -   | rd        | [rs1]        |
| 001    | 10            | 0         | DOUBLE_RS2  | -   | rs2 | -   | rd        | [rs2]        |
| 001    | 11            | 0         | ADD_MULTI   | rs1 | rs2 | -   | rd        | [rs1, rs2]   |
| 001    | 00            | 1         | ADD_RS3_R   | rs1 | rs2 | rs3 | rd        | [rs1, rs2, rs3] |

### R4 opcodes (FP-style, offloadable when FP disabled)

| Opcode | funct2[26:25] | Instruction | rs1 | rs2 | rs3 | Writeback | register_read |
|--------|--------------|-------------|-----|-----|-----|-----------|--------------|
| MADD  (0x43) | 00 | MADD_RS3_R4 | rs1 | rs2 | rs3 | rd | [rs1, rs2, rs3] |
| MSUB  (0x47) | 00 | MSUB_RS3_R4 | rs1 | rs2 | rs3 | rd | [rs1, rs2, rs3] |
| NMSUB (0x4B) | 00 | NMSUB_RS3_R4 | rs1 | rs2 | rs3 | rd | [rs1, rs2, rs3] |
| NMADD (0x4F) | 00 | NMADD_RS3_R4 | rs1 | rs2 | rs3 | rd | [rs1, rs2, rs3] |
