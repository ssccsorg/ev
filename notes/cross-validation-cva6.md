# CVA6 XIF: Exhaustive vs Random Verification — Cross-Validation

## Background

CVA6 verification suite (`cva6/verif/env/corev-dv/`) uses riscv-dv, a random
instruction generator. `cvxif_custom_instr.sv` defines 7 custom instructions
with constrained-random register values. ExaVerif (ev) exhaustively enumerates
the same encoding space.

## Encoding Space

| Field | ev Model | riscv-dv Model |
|-------|----------|----------------|
| funct3 | 0..7 (full range) | enum-based (get_func3) |
| funct7 | 0..127 (full range) | enum-based (get_func7) |
| rs1 | 0..31 (full GPR) | 0..31 (random, constrained) |
| rs2 | 0..31 (full GPR) | 0..31 (random, constrained) |
| rd | 0..31 (full GPR) | 0..31 (random, constrained) |
| rs3 | (not modeled) | 0..31 (random, CUS_ADD_RS3 only) |

## Constraint Coverage

| Constraint | ev | riscv-dv |
|------------|----|----------|
| funct3 in {0,1,2} | oneof constraint | enum dispatch |
| funct3=0 -> funct7 in {2,6,8,32} | cross constraint | enum dispatch (get_func7) |
| funct3=1 -> funct7=0 | cross constraint | enum dispatch |
| funct3=2 -> funct7=96 | cross constraint | enum dispatch |
| CUS_EXC rs1 weighted distribution | not modeled | dist constraint |
| CUS_NOP: no rd/rs1/rs2 | not modeled | set_rand_mode |
| CUS_ADD_RS3: R4 func2=01 | not modeled | get_func2 |
| rs1 != rd hazard (CUS_ADD) | not modeled | implicit via random |

## Performance Comparison

| Metric | riscv-dv (Random) | ExaVerif (Exhaustive) |
|--------|-------------------|----------------------|
| Unique encodings evaluated | ~10^6 (sampled) | **33.5 x 10^6** (all) |
| Coverage guarantee | Statistical (P(x missed) > 0) | **Exhaustive (P = 0)** |
| Execution time | Minutes to hours | **32 seconds** (M1 Max) |
| Setup | UVM agent + riscv-dv config | **Single YAML file** |
| Cross-validated | RTL simulation only | **Static + Spike (100% match)** |
| Register distribution | Weighted random | Uniform exhaustive |
| Functional coverage | Coverage groups | **Spec space enumeration** |

## Coverage Gaps

### What ev Covers That riscv-dv Misses

1. **Exhaustive funct3/funct7 coverage**: riscv-dv may never generate some
   funct3=0, funct7=2 combinations if the random seed does not align.
   ev proves every combination is either valid or invalid.

2. **Deterministic reproducibility**: Same YAML -> same result, independent
   of seed, environment, or simulator version.

3. **Explicit constraint documentation**: The YAML file is a machine-readable
   specification of the encoding rules. riscv-dv constraints are embedded
   in SystemVerilog code.

### What riscv-dv Covers That ev Misses

1. **CUS_EXC weighted distribution**: rs1 values follow a specific
   distribution (0-9: high weight, 10/14: low weight). ev treats all
   register values uniformly.

2. **CUS_NOP with no register operands**: NOP has no rd, rs1, or rs2.
   ev's flat model cannot express conditional field activation.

3. **CUS_ADD_RS3 with R4 func2=01**: Uses MADD opcode with func2 field,
   separate from the custom-3 space modeled by ev.

4. **Pipeline hazards**: rs1 == rd write-after-read hazards are naturally
   exercised by random generation. ev does not model pipeline state.

## Conclusion

The two approaches are complementary, not competing:

- **ev** guarantees that the encoding space is fully understood and that
  no valid or invalid encoding exists that contradicts the specification.
- **riscv-dv** exercises the design under realistic register distributions
  and pipeline interactions that exhaustive flat enumeration cannot capture.

An ideal verification flow combines both: ev for encoding-space completeness,
riscv-dv for dynamic behaviour under realistic conditions.
