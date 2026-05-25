#!/usr/bin/env bash
set -euo pipefail
#
# demo-ssccs-poc.sh — Channel: ev ↔ SSCCS POC golden anchor cross-verification
#
# Clones ssccs, extracts golden anchors from observe_full.S, generates
# YAML fixtures, and runs ev check to independently verify that the
# exhaustive constraint engine produces the same results as the
# hand-written RISC‑V assembly.
#
# Channels verified:
#   narrow   — even ∧ range_0_10, proj_id  (5 segments, 2 pass)
#   broad    — no constraints, proj_id      (5 segments, 5 pass)
#   sum3d_a  — proj_sum3d on (2,1,0)       (1 segment)
#   sum3d_b  — proj_sum3d on (1,2,3)       (1 segment)
#   parity   — proj_parity on {2,3}         (2 segments)
#
# Usage:
#   ./scripts/demo-ssccs-poc.sh
#

cd "$(dirname "$0")/.."

TMPDIR="${TMPDIR:-/tmp}"
WORKDIR="$TMPDIR/ev-demo-$$"
SSCCS_DIR="$WORKDIR/ssccs"
PASSED=0
FAILED=0

cleanup() { rm -rf "$WORKDIR"; }
trap cleanup EXIT

# ── Step 1: Clone ssccs2 ──────────────────────────────────────────────

echo "=== Channel Demo: ev ↔ SSCCS POC ==="
echo ""
echo "Step 1: Cloning ssccs..."
mkdir -p "$WORKDIR"
git clone --depth 1 https://github.com/ssccsorg/ssccs.git "$SSCCS_DIR" 2>&1 | tail -1
ASM="$SSCCS_DIR/poc/baremetal_riscv/asm/observe_full.S"

if [ ! -f "$ASM" ]; then
    echo "ERROR: observe_full.S not found"
    exit 1
fi
echo "  ✓ cloned"
echo ""

# ── Step 2: Extract golden anchors ────────────────────────────────────

echo "Step 2: Extracting golden anchors..."
parse_golden() {
    grep "GOLDEN_${1}:" "$ASM" | head -1 | sed "s/.*GOLDEN_${1}: *//" | tr -d ' '
}

SEGMENTS=$(parse_golden "SEGMENTS")
NARROW=$(parse_golden "NARROW")
BROAD=$(parse_golden "BROAD")
SUM3D_A=$(parse_golden "SUM3D_A")
SUM3D_B=$(parse_golden "SUM3D_B")
PARITY_2=$(parse_golden "PARITY_2")
PARITY_3=$(parse_golden "PARITY_3")

echo "  SEGMENTS:  $SEGMENTS"
echo "  NARROW:    $NARROW    (even ∧ range_0_10)"
echo "  BROAD:     $BROAD       (no constraints)"
echo "  SUM3D_A:   $SUM3D_A         (2,1,0 → sum)"
echo "  SUM3D_B:   $SUM3D_B         (1,2,3 → sum)"
echo "  PARITY:    $PARITY_2,$PARITY_3       (2→even, 3→odd)"
echo ""

# ── Step 3: Generate YAML fixtures ────────────────────────────────────

YAML_DIR="$WORKDIR/fixtures"
mkdir -p "$YAML_DIR"
IFS=',' read -ra SEGS <<< "$SEGMENTS"

# Narrow: 5 segments, even AND range_0_10, proj_id
cat > "$YAML_DIR/narrow.yaml" << YAML
target: ssccs_poc_narrow
fields:
  coord:
    values: [${SEGS[0]}, ${SEGS[1]}, ${SEGS[2]}, ${SEGS[3]}, ${SEGS[4]}]
constraints:
  - type: even
    axis: 0
  - type: range
    axis: 0
    min: 0
    max: 10
projector:
  type: identity
YAML

# Broad: 5 segments, no constraints, proj_id
cat > "$YAML_DIR/broad.yaml" << YAML
target: ssccs_poc_broad
fields:
  coord:
    values: [${SEGS[0]}, ${SEGS[1]}, ${SEGS[2]}, ${SEGS[3]}, ${SEGS[4]}]
projector:
  type: identity
YAML

# Sum3D A: single point (2,1,0)
cat > "$YAML_DIR/sum3d_a.yaml" << YAML
target: ssccs_poc_sum3d_a
fields:
  x: { values: [2] }
  y: { values: [1] }
  z: { values: [0] }
projector: { type: sum }
YAML

# Sum3D B: single point (1,2,3)
cat > "$YAML_DIR/sum3d_b.yaml" << YAML
target: ssccs_poc_sum3d_b
fields:
  x: { values: [1] }
  y: { values: [2] }
  z: { values: [3] }
projector: { type: sum }
YAML

# Parity: 2 segments
cat > "$YAML_DIR/parity.yaml" << YAML
target: ssccs_poc_parity
fields:
  coord:
    values: [2, 3]
projector:
  type: parity
YAML

echo "Step 3: YAML fixtures generated"
for f in "$YAML_DIR"/*.yaml; do
    echo "  $(basename "$f")"
done
echo ""

# ── Step 4: Build ev ──────────────────────────────────────────────────

echo "Step 4: Building ev..."
cargo build --release --quiet 2>&1
echo "  ✓ built"
echo ""

EV="./target/release/ev"

# ── Step 5: Run channels ──────────────────────────────────────────────

run_channel() {
    local name="$1"; local yaml="$2"; local golden="$3"
    echo "--- Channel: $name ---"
    echo "  YAML:   $(basename "$yaml")"
    echo "  Golden: $golden"

    local output ev_fmt
    set +e
    output=$("$EV" check --target "$yaml" --json 2>&1)
    local ec=$?
    set -e

    ev_fmt=$(echo "$output" | python3 -c "
import json, sys
data = json.load(sys.stdin)
vals = []
for r in data['results']:
    if r['passed']:
        vals.append(str(r['projection']))
    else:
        vals.append('REJECT')
print(','.join(vals))
" 2>/dev/null)

    echo "  ev:      $ev_fmt"

    if [ "$ev_fmt" = "$golden" ]; then
        echo "  ✓ MATCH"
        PASSED=$((PASSED + 1))
    else
        echo "  ✗ MISMATCH (expected: $golden, got: $ev_fmt)"
        FAILED=$((FAILED + 1))
    fi
    echo ""
}

run_channel "narrow"   "$YAML_DIR/narrow.yaml"   "$NARROW"
run_channel "broad"    "$YAML_DIR/broad.yaml"    "$BROAD"
run_channel "sum3d_a"  "$YAML_DIR/sum3d_a.yaml"  "$SUM3D_A"
run_channel "sum3d_b"  "$YAML_DIR/sum3d_b.yaml"  "$SUM3D_B"
run_channel "parity"   "$YAML_DIR/parity.yaml"   "$PARITY_2,$PARITY_3"

# ── Summary ───────────────────────────────────────────────────────────

echo "══════════════════════════════════════"
echo "  Channel Demo Summary"
echo "══════════════════════════════════════"
echo ""
echo "  Passed: $PASSED / 5"
echo "  Failed: $FAILED"
echo ""

if [ "$FAILED" -eq 0 ]; then
    echo "  All 5 channels match POC golden anchors."
    echo "  ev independently reproduces RISC‑V assembly results."
    echo ""
    echo "  narrow:   even ∧ range_0_10  →  $NARROW"
    echo "  broad:    no constraints     →  $BROAD"
    echo "  sum3d_a:  (2,1,0)            →  $SUM3D_A"
    echo "  sum3d_b:  (1,2,3)            →  $SUM3D_B"
    echo "  parity:   {2,3}              →  $PARITY_2,$PARITY_3"
    echo ""
    echo "══════════════════════════════════════"
    exit 0
else
    echo "  $FAILED channel(s) failed."
    echo "══════════════════════════════════════"
    exit 1
fi
