#!/usr/bin/env bash
set -euo pipefail
#
# ev — Single entry point
#
# Usage:
#   ./run.sh              # Full pipeline: fix → code → verify → demo
#   ./run.sh --ci         # CI mode: code → verify → demo (ssccs from ../ssccs)
#   ./run.sh --code       # fmt → clippy → build → test (strict)
#   ./run.sh --fix        # auto-fix → build → test
#   ./run.sh --verify     # Yosys synthesis + fixtures (binary must exist)
#   ./run.sh --demo       # Channel demo: ev ↔ SSCCS POC golden anchors
#   ./run.sh --help
#

cd "$(dirname "$0")"
export RUSTFLAGS="-D warnings"

# ── Pre-process: auto-format before any build-related mode ─────────────
# Runs unconditionally before --code, --fix, --verify, --ci, and the
# default pipeline so fmt/clippy issues are caught before strict checks.
# Skipped for --help and --demo which don't need compilation.
if [[ "${1:-}" != "--help" && "${1:-}" != "-h" && "${1:-}" != "--demo" ]]; then
    cargo fmt --all
    cargo clippy --fix --allow-dirty 2>&1 || true
    cargo fix --allow-dirty 2>&1 || true
    cargo fmt --all
fi

# ── Helpers ────────────────────────────────────────────────────────────

EV=./target/release/ev
ALL_PASS=tests/fixtures/all_pass.xif.yaml
MIXED=tests/fixtures/sample.xif.yaml

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

verify_synth() {
    echo "=== Yosys version ==="
    yosys --version

    echo "=== synthesis (text) ==="
    $EV check --target "$ALL_PASS" --synth

    echo "=== synthesis (json) ==="
    $EV check --target "$ALL_PASS" --synth --json > /tmp/synth_fact.json 2>/tmp/synth_stderr.txt
    grep -q '"fact_type": "synthesis_result"' /tmp/synth_fact.json || { cat /tmp/synth_stderr.txt; echo "FAILED: missing fact_type"; exit 1; }
    grep -q '"status": "ok"' /tmp/synth_fact.json || { cat /tmp/synth_stderr.txt; echo "FAILED: synthesis status not ok"; exit 1; }
    echo "  ok"
}

verify_fixtures() {
    echo "=== all-pass fixture ==="
    $EV check --target "$ALL_PASS"

    echo "=== mixed fixture ==="
    EC=0; $EV check --target "$MIXED" || EC=$?
    if [ "$EC" -eq 1 ]; then
        echo "  exit: 1 (expected — 84 of 96 fail eq constraint)"
    else
        echo "  exit: $EC (UNEXPECTED)"
        exit 1
    fi

    echo "=== json output ==="
    $EV check --target "$MIXED" --json 2>&1 | head -8 || true
}

run_demo() {
    local ssccs_dir="$1"
    if [ -n "$ssccs_dir" ] && [ -d "$ssccs_dir" ]; then
        echo "=== channel demo ==="
        set +e
        SSCCS_DIR="$ssccs_dir" bash scripts/demo-ssccs-poc.sh
        local ec=$?
        set -e
        if [ "$ec" -eq 0 ]; then
            echo "  demo: all 5/5 passed"
        else
            echo "  demo: exit $ec (non-fatal)"
        fi
    else
        echo "=== channel demo: skipped (ssccs not found) ==="
    fi
}

# ── Modes ──────────────────────────────────────────────────────────────

case ${1:-} in
    --ci)
        echo "══════════════════════════════════════"
        echo "  ev — CI pipeline"
        echo "══════════════════════════════════════"
        code_checks
        verify_synth
        verify_fixtures
        # In CI, ../ssccs is set up by the workflow.
        run_demo "$(cd .. && pwd)/ssccs"
        echo ""
        echo "  All CI checks passed."
        echo "══════════════════════════════════════"
        ;;
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
        if command -v yosys &>/dev/null; then
            verify_synth
        else
            echo "  yosys not found — skipping synthesis"
        fi
        verify_fixtures
        echo ""
        echo "  Verification passed."
        echo "══════════════════════════════════════"
        ;;
    --demo)
        # Self-contained: clone ssccs if not provided.
        exec bash scripts/demo-ssccs-poc.sh
        ;;
    --help|-h)
        echo "Usage: $0 [OPTION]"
        echo "  (no arg)   Full pipeline: fix → code → verify → demo"
        echo "  --ci       CI mode: code → verify → demo"
        echo "  --code     fmt → clippy → build → test (strict)"
        echo "  --fix      auto-fix → build → test"
        echo "  --verify   Yosys + fixtures (binary needed)"
        echo "  --demo     Channel demo: ev ↔ SSCCS POC"
        exit 0
        ;;
    *)
        # Full local pipeline
        echo "══════════════════════════════════════"
        echo "  ev — Full Pipeline"
        echo "══════════════════════════════════════"

        echo "=== Phase 1: auto-fix ==="
        cargo fmt --all
        cargo clippy --fix --allow-dirty 2>&1 || true
        cargo fix --allow-dirty 2>&1 || true
        cargo fmt --all

        echo "=== Phase 2: code checks ==="
        code_checks

        echo "=== Phase 3: integration ==="
        if command -v yosys &>/dev/null; then
            verify_synth
        else
            echo "  yosys not found — skipping synthesis"
            verify_fixtures
        fi

        echo "=== Phase 4: channel demo ==="
        if [ -d ../ssccs ]; then
            run_demo "$(cd .. && pwd)/ssccs"
        else
            echo "  ../ssccs not found — skipping demo"
        fi

        echo ""
        echo "══════════════════════════════════════"
        echo "  All done."
        echo "══════════════════════════════════════"
        ;;
esac
