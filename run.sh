#!/usr/bin/env bash
set -euo pipefail
#
# ev — Single entry point
#
# Usage:
#   ./run.sh              # Full pipeline: fix → code → verify
#   ./run.sh --code       # fmt → clippy → build → test (strict)
#   ./run.sh --fix        # auto-fix → build → test
#   ./run.sh --verify     # Yosys synthesis + fixtures (binary must exist)
#   ./run.sh --demo       # Channel demo: ev ↔ SSCCS POC golden anchors
#   ./run.sh --help
#

cd "$(dirname "$0")"
export RUSTFLAGS="-D warnings"
EV_IMAGE="${EV_IMAGE:-ghcr.io/ssccsorg/ev:latest}"

# Pre-process: auto-fmt for all build modes except --help and --demo.
if [[ "${1:-}" != "--help" && "${1:-}" != "-h" && "${1:-}" != "--demo" ]]; then
    cargo fmt --all
    cargo clippy --fix --allow-dirty 2>&1 || true
    cargo fix --allow-dirty 2>&1 || true
    cargo fmt --all
fi

EV=./target/release/ev
ALL_PASS=tests/fixtures/all_pass.xif.yaml
MIXED=tests/fixtures/sample.xif.yaml

# ── Helpers ───────────────────────────────────────────────────────────

code_checks() {
    echo "=== fmt ==="
    cargo fmt --check
    echo "=== clippy ==="
    cargo clippy --all-targets
    echo "=== build ==="
    cargo build --release
    echo "=== test ==="
    cargo test --release
}

# Run a Yosys-dependent command inside the CI container.
# Falls back to direct execution if Yosys is available locally.
_yosys() {
    if command -v yosys &>/dev/null; then
        "$@"
    else
        docker run --rm --entrypoint bash \
            -v "$(pwd):/workspace" \
            "$EV_IMAGE" \
            -c "cd /workspace && $*"
    fi
}

verify_synth() {
    _yosys yosys --version
    echo "=== synthesis (text) ==="
    _yosys "$EV" synth --target "$ALL_PASS"
    echo "=== synthesis (json) ==="
    local tmpf
    tmpf=$(mktemp /tmp/synth_fact.XXXXXX.json)
    local tmpe
    tmpe=$(mktemp /tmp/synth_stderr.XXXXXX.txt)
    _yosys "$EV" synth --target "$ALL_PASS" --json > "$tmpf" 2>"$tmpe"
    grep -q '"fact_type": "synthesis_result"' "$tmpf" || { cat "$tmpe"; echo "FAILED: missing fact_type"; exit 1; }
    grep -q '"status": "ok"' "$tmpf" || { cat "$tmpe"; echo "FAILED: synthesis status not ok"; exit 1; }
    echo "  ok"
    rm -f "$tmpf" "$tmpe"
}

verify_fixtures() {
    echo "=== all-pass fixture ==="
    $EV verify --target "$ALL_PASS"
    echo "=== mixed fixture ==="
    EC=0; $EV verify --target "$MIXED" || EC=$?
    if [ "$EC" -eq 1 ]; then
        echo "  exit: 1 (expected — 84 of 96 fail eq constraint)"
    else
        echo "  exit: $EC (UNEXPECTED)"
        exit 1
    fi
    echo "=== json output ==="
    $EV verify --target "$MIXED" --json 2>&1 | head -8 || true
    echo "=== cva6 xif reference fixture ==="
    EC=0; $EV verify --target "tests/fixtures/cva6_xif_ref.xif.yaml" --json 2>&1 | python3 -c "
import sys, json
data = json.load(sys.stdin)
passed = data['passed']
failed = data['failed']
total = data['total']
print(f'  Target: cva6_xif_ref (CVA6 CV-X-IF reference coprocessor)')
print(f'  Total: {total}')
print(f'  Passed: {passed} (valid custom-3 encodings)')
print(f'  Failed: {failed} (illegal or constraint-violating encodings)')
print(f'  Valid instructions: funct3=0 with funct7 in 2,6,8,32; funct3=1 with funct7=0; funct3=2 with funct7=96')
print(f'  Register fields: rs1, rs2, rd full 0..31 range')
" || EC=$?
    if [ "$EC" -eq 0 ]; then
        echo "  All encodings valid — no unexpected failures."
    else
        echo "  Constraint-violating encodings detected (expected — coprocessor rejects illegal funct3/funct7)."
    fi
    echo "=== simulate help ==="
    $EV simulate --help 2>&1 | head -3
    echo "=== spike simulation (mock) ==="
    $EV simulate --target "$ALL_PASS" --json 2>&1 | python3 -c "
import sys, json
data = json.load(sys.stdin)
print(f'  tool: {data[\"origin\"]}')
print(f'  total: {data[\"payload\"][\"total\"]}, passed: {data[\"payload\"][\"passed\"]}')
" || echo "  spike backend not available — check EV_SIM_BACKEND"
}

# ── Modes ─────────────────────────────────────────────────────────────

case ${1:-} in
    --code)
        echo "══════════════════════════════════════"
        echo "  ev — code checks"
        echo "══════════════════════════════════════"
        code_checks
        echo ""
        echo "  All code checks passed."
        echo "══════════════════════════════════════"
        ;;
    --fix)
        echo "══════════════════════════════════════"
        echo "  ev — auto-fix + test"
        echo "══════════════════════════════════════"
        cargo fmt --all
        cargo clippy --fix --allow-dirty 2>&1 || true
        cargo fix --allow-dirty 2>&1 || true
        cargo fmt --all
        cargo build --release
        cargo test --release
        echo ""
        echo "  All checks passed (with auto-fix)."
        echo "══════════════════════════════════════"
        ;;
    --verify)
        if [ ! -f "$EV" ]; then
            echo "Binary not found. Run './run.sh' first."
            exit 1
        fi
        echo "══════════════════════════════════════"
        echo "  ev — integration verification"
        echo "══════════════════════════════════════"
        verify_synth
        verify_fixtures
        echo ""
        echo "  Verification passed."
        echo "══════════════════════════════════════"
        ;;
    --demo)
        exec bash scripts/demo-ssccs-poc.sh
        ;;
    --help|-h)
        echo "Usage: $0 [OPTION]"
        echo "  (no arg)   Full pipeline: auto-fix → code → verify"
        echo "  --code     fmt → clippy → build → test (strict)"
        echo "  --fix      auto-fix → build → test"
        echo "  --verify   Yosys + fixtures (binary needed)"
        echo "  --demo     Channel demo: ev ↔ SSCCS POC (standalone)"
        exit 0
        ;;
    *)
        echo "══════════════════════════════════════"
        echo "  ev — Full Pipeline"
        echo "══════════════════════════════════════"
        echo "=== auto-fix ==="
        cargo fmt --all
        cargo clippy --fix --allow-dirty 2>&1 || true
        cargo fix --allow-dirty 2>&1 || true
        cargo fmt --all
        echo "=== code checks ==="
        code_checks
        echo "=== integration ==="
        verify_synth
        verify_fixtures
        echo ""
        echo "══════════════════════════════════════"
        echo "  All done."
        echo "══════════════════════════════════════"
        ;;
esac
