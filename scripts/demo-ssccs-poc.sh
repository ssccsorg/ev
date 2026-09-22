#!/usr/bin/env bash
set -euo pipefail
#
# demo-ssccs-poc.sh — Channel: ev ↔ SSCCS POC golden anchor cross-verification
#
# Clones ssccs (or uses an existing SSCCS_DIR), reads the golden anchors that
# the hand-written RISC-V assembly in observe_full.S computes for each
# segment, generates YAML fixtures, and runs `ev verify` to confirm that the
# exhaustive constraint engine reproduces those per-segment results.
#
# Channels verified:
#   narrow   — even ∧ 0..10, identity        (5 segments, 2 pass)
#   broad    — no constraints, identity      (5 segments, 5 pass)
#   sum3d_a  — sum over (2,1,0)              (1 point)
#   sum3d_b  — sum over (1,2,3)              (1 point)
#   parity   — parity over {2,3}             (2 points)
#
# The constraints in the generated fixtures are runtime constraints (even,
# ge, le). `ev verify` reports the structurally valid subset, so a structural
# constraint such as `range` would remove a segment from the report instead
# of marking it rejected, and the rows would no longer line up with the
# assembly's per-segment golden list.
#
# Usage:
#   ./run.sh --demo
#   SSCCS_DIR=../ssccs bash scripts/demo-ssccs-poc.sh   (skip the clone)
#

cd "$(dirname "$0")/.."

PASSED=0
FAILED=0

if [ -z "${SSCCS_DIR:-}" ]; then
    TMPDIR="${TMPDIR:-/tmp}"
    WORKDIR="$TMPDIR/ev-demo-$$"
    SRCDIR="$WORKDIR/ssccs"
    cleanup() { rm -rf "$WORKDIR"; }
    trap cleanup EXIT
    echo "=== Channel Demo: ev ↔ SSCCS POC ==="
    echo ""
    echo "Step 1: Cloning ssccs..."
    mkdir -p "$WORKDIR"
    git clone --depth 1 https://github.com/ssccsorg/ssccs.git "$SRCDIR" 2>&1 | tail -1
    YAML_DIR="$WORKDIR/fixtures"
else
    SRCDIR="$SSCCS_DIR"
    YAML_DIR="${SSCCS_DIR}-fixtures"
    echo "=== Channel Demo: ev ↔ SSCCS POC ==="
    echo ""
    echo "Step 1: Using existing ssccs at $SSCCS_DIR"
fi

ASM="$SRCDIR/poc/baremetal_riscv/asm/observe_full.S"

if [ ! -f "$ASM" ]; then
    echo "ERROR: observe_full.S not found at $ASM"
    exit 1
fi
echo "  ok"
echo ""

# ── Step 2: Extract golden anchors ────────────────────────────────────

echo "Step 2: Extracting golden anchors from observe_full.S..."
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
echo "  NARROW:    $NARROW    (even and 0..10)"
echo "  BROAD:     $BROAD       (no constraints)"
echo "  SUM3D_A:   $SUM3D_A         (2,1,0 -> sum)"
echo "  SUM3D_B:   $SUM3D_B         (1,2,3 -> sum)"
echo "  PARITY:    $PARITY_2,$PARITY_3       (2 -> even, 3 -> odd)"
echo ""

# ── Step 3: Generate YAML fixtures ────────────────────────────────────

mkdir -p "$YAML_DIR"
IFS=',' read -ra SEGS <<< "$SEGMENTS"

# Narrow: 5 segments, even and 0..10, identity projection.
cat > "$YAML_DIR/narrow.yaml" << YAML
target: ssccs_poc_narrow
fields:
  coord:
    values: [${SEGS[0]}, ${SEGS[1]}, ${SEGS[2]}, ${SEGS[3]}, ${SEGS[4]}]
constraints:
  - type: even
    field: "coord"
  - type: ge
    field: "coord"
    value: 0
  - type: le
    field: "coord"
    value: 10
projector:
  type: identity
  field: "coord"
YAML

# Broad: 5 segments, no constraints, identity projection.
cat > "$YAML_DIR/broad.yaml" << YAML
target: ssccs_poc_broad
fields:
  coord:
    values: [${SEGS[0]}, ${SEGS[1]}, ${SEGS[2]}, ${SEGS[3]}, ${SEGS[4]}]
projector:
  type: identity
  field: "coord"
YAML

# Sum3D A: single point (2,1,0).
cat > "$YAML_DIR/sum3d_a.yaml" << YAML
target: ssccs_poc_sum3d_a
fields:
  x: { values: [2] }
  y: { values: [1] }
  z: { values: [0] }
projector: { type: sum }
YAML

# Sum3D B: single point (1,2,3).
cat > "$YAML_DIR/sum3d_b.yaml" << YAML
target: ssccs_poc_sum3d_b
fields:
  x: { values: [1] }
  y: { values: [2] }
  z: { values: [3] }
projector: { type: sum }
YAML

# Parity: the two segments the assembly classifies.
cat > "$YAML_DIR/parity.yaml" << YAML
target: ssccs_poc_parity
fields:
  coord:
    values: [2, 3]
projector:
  type: parity
  field: "coord"
YAML

echo "Step 3: YAML fixtures generated"
for f in "$YAML_DIR"/*.yaml; do
    echo "  $(basename "$f")"
done
echo ""

# ── Step 4: Build ev ──────────────────────────────────────────────────

echo "Step 4: Building ev..."
cargo build --release --quiet 2>&1
echo "  ok"
echo ""

EV="./target/release/ev"

# ── Step 5: Run channels ──────────────────────────────────────────────

# Decode the Fact envelope that `ev verify --format json` prints. The payload
# is an opaque byte vector carrying the VerificationReport JSON; each result
# row contributes its projection, or REJECT when the combination failed.
CHANNEL_ROWS=$(cat <<'PY'
import json, sys

fact = json.load(sys.stdin)
report = json.loads(bytes(fact["payload"]).decode())
rows = [str(r["projection"]) if r["passed"] else "REJECT" for r in report["results"]]
print(",".join(rows))
PY
)

run_channel() {
    local name="$1"; local yaml="$2"; local golden="$3"
    echo "--- Channel: $name ---"
    echo "  YAML:   $(basename "$yaml")"
    echo "  Golden: $golden"

    local output ev_fmt
    set +e
    output=$("$EV" verify --target "$yaml" --format json 2>&1)
    set -e

    ev_fmt=$(echo "$output" | python3 -c "$CHANNEL_ROWS" 2>/dev/null) || ev_fmt=""

    if [ -z "$ev_fmt" ]; then
        echo "  FAILED: could not read the result rows from ev output"
        echo "$output" | head -5 | sed 's/^/    /'
        FAILED=$((FAILED + 1))
        echo ""
        return
    fi

    echo "  ev:     $ev_fmt"

    if [ "$ev_fmt" = "$golden" ]; then
        echo "  MATCH"
        PASSED=$((PASSED + 1))
    else
        echo "  MISMATCH (expected: $golden, got: $ev_fmt)"
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
    echo "  All 5 channels match the POC golden anchors."
    echo "  ev independently reproduces the RISC-V assembly results."
    echo ""
    echo "  narrow:   even and 0..10   ->  $NARROW"
    echo "  broad:    no constraints   ->  $BROAD"
    echo "  sum3d_a:  (2,1,0)          ->  $SUM3D_A"
    echo "  sum3d_b:  (1,2,3)          ->  $SUM3D_B"
    echo "  parity:   {2,3}            ->  $PARITY_2,$PARITY_3"
    echo ""
    echo "══════════════════════════════════════"
    exit 0
else
    echo "  $FAILED channel(s) failed."
    echo "══════════════════════════════════════"
    exit 1
fi
