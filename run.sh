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
    local tmpf tmp_suffix
    tmp_suffix=$(date +%s%N)
    tmpf="/tmp/synth_fact_${tmp_suffix}.json"
    local tmpe
    tmpe="/tmp/synth_stderr_${tmp_suffix}.txt"
    _yosys "$EV" synth --target "$ALL_PASS" --json > "$tmpf" 2>"$tmpe"
    # Fact envelope must contain fact_type; status is inside payload
    grep -q '"fact_type": "synthesis_result"' "$tmpf" || { cat "$tmpe"; echo "FAILED: missing fact_type"; exit 1; }
    # Check that payload is non-empty and contains status='ok'
    python3 -c "import json,sys; d=json.load(open('$tmpf')); p=json.loads(bytes(d['payload']).decode()); assert p['status']=='ok', f'status: {p[\"status\"]}'" || { cat "$tmpe"; echo "FAILED: synthesis status not ok"; exit 1; }
    echo "  ok"
    rm -f "$tmpf" "$tmpe"
}

check_spike() {
    if command -v spike &>/dev/null && command -v riscv64-unknown-elf-gcc &>/dev/null; then
        return 0
    fi
    return 1
}

verify_sim() {
    if ! check_spike; then
        echo "  Spike or RISC-V toolchain not found — skipping simulation tests."
        return 0
    fi
    echo "=== spike simulation (all_pass fixture) ==="
    EV_SIM_BACKEND=spike EV_PK_PATH="${EV_PK_PATH:-pk}" \
        "$EV" simulate --target "$ALL_PASS" 2>&1
    echo "=== spike simulation (sample fixture) ==="
    EV_SIM_BACKEND=spike EV_PK_PATH="${EV_PK_PATH:-pk}" \
        "$EV" simulate --target "$MIXED" 2>&1 || true
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
    echo "  $($EV verify --target "$MIXED" --json 2>/dev/null | python3 -c 'import sys,json;d=json.load(sys.stdin);p=json.loads(bytes(d["payload"]).decode());print(f"Total: {p["total"]}, Passed: {p["passed"]}, Failed: {p["failed"]}")' 2>/dev/null || echo 'parse error')"
    echo "=== cva6 xif reference fixture (33M combos) ==="
    echo "  Skipped in default mode. Run 'bash run.sh --verify' to include."
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
        verify_sim
        echo "=== cva6 xif ref fixture (33M combos, text output) ==="
        EC=0; $EV verify --target "tests/fixtures/cva6_xif_ref.xif.yaml" 2>&1 | tail -4
        echo "=== cva6 xif encoding-only spike sim ==="
        EV_SIM_BACKEND=spike EV_PK_PATH="${EV_PK_PATH:-pk}" \
            "$EV" simulate --target "tests/fixtures/cva6_xif_encoding.xif.yaml" 2>&1 | tail -3
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
        verify_sim
        echo ""
        echo "══════════════════════════════════════"
        echo "  All done."
        echo "══════════════════════════════════════"
        ;;
esac
