# CVA6 XIF: ev Exhaustive vs riscv-dv Random — Cross-Validation

## Background

CVA6's verification suite (`cva6/verif/env/corev-dv/`) uses riscv-dv, a random
instruction generator. `cvxif_custom_instr.sv` defines 7 custom instructions
with constrained-random register values. ev exhaustively enumerates the same
encoding space.

## Encoding Space

| Field | ev model | riscv-dv model |
|-------|----------|---------------|
| funct3 | 0..7 (range) | enum-based (get_func3) |
| funct7 | 0..127 (range) | enum-based (get_func7) |
| rs1 | 0..7 (range) | 0..31 (random, constrained) |
| rs2 | 0..7 (range) | 0..31 (random, constrained) |
| rd | 0..7 (range) | 0..31 (random, constrained) |
| rs3 | (not modeled) | 0..31 (random, only ADD_RS3_R) |

## Constraint Coverage

| Constraint | ev | riscv-dv |
|------------|----|----------|
| funct3 ∈ {0,1,2} | oneof ✓ | enum ✓ |
| funct3=0 → funct7 ∈ {2,6,8} | cross ✓ | enum ✓ |
| funct3=1 → funct7=0 | cross ✓ | enum ✓ |
| funct3=2 → funct7=96 | cross ✓ | enum ✓ |
| CUS_EXC rs1 distribution | — | dist constraint |
| CUS_NOP: no rd/rs1/rs2 | — | set_rand_mode |
| CUS_ADD_RS3: R4 opcode | — | get_func2 |

## Coverage Gap Analysis

### ev가 커버하는 영역 (riscv-dv보다 우위)

1. **Exhaustive**: 모든 funct3/funct7/rs1/rs2/rd 조합 — riscv-dv는 random sampling
2. **명시적 constraint 문서화**: oneof + cross로 encoding 규칙이 YAML에 명시
3. **검증 결과의 재현성**: 동일 입력 → 항상 동일 출력

### riscv-dv가 커버하는 영역 (ev보다 우위)

1. **CUS_EXC 분포**: rs1 값에 weighted distribution 적용 (0-9:10, 10:2, 11-13:10, 14:2, 15:10, 16-23:2, 25-31:2)
2. **CUS_NOP의 rd/rs1/rs2 비활성화**: 조건부 필드 마스킹 (ev는 flat model)
3. **CUS_ADD_RS3의 R4 opcode**: func2=01 사용, rs3 필드 추가
4. **레지스터 0..31 전체 범위**: ev는 0..7로 축소 (MAX_COMBINATIONS 제한)

## 결론

두 접근법은 **상호보완적**:

- ev: exhaustive encoding space 검증 (funct3/funct7 조합의 완전성)
- riscv-dv: weighted random으로 edge case 탐색 (레지스터 분포, pipeline hazard)

ev가 아직 모델링하지 않은 영역:

- R4 opcode (CUS_ADD_RS3)
- 조건부 필드 활성화 (CUS_NOP은 rs1/rs2/rd 없음)
- weighted distribution constraint
- full register range (32 values)
