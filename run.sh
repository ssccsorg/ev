#!/usr/bin/env bash
set -euo pipefail
#
# ev — Local CI runner
#
# Mirrors .github/workflows/build-ev.yml locally.
# Pre-flight auto-fixes catch formatting, trivial clippy, and compiler
# suggestions before strict checks — eliminating most CI noise.
#
# Pipeline:
#   fmt → clippy --fix → fix → fmt → check → clippy → test → verify
#
# The verify step runs `ev check` against sample fixtures to confirm
# the full YAML → domain expansion → observe → report pipeline works
# for both all-pass and mixed-pass/fail scenarios.
#
# Usage:
#   ./run.sh              # Full pipeline (default)
#   ./run.sh --check      # fmt + check only
#   ./run.sh --clippy     # full lint cycle (fix → fmt → clippy)
#   ./run.sh --test       # fmt + test + verify
#   ./run.sh --verify     # run ev check against fixtures only
#

cd "$(dirname "$0")"

MODE="all"

while [[ "$#" -gt 0 ]]; do
    case $1 in
        --check)  MODE="check" ;;
        --clippy) MODE="clippy" ;;
        --test)   MODE="test" ;;
        --verify) MODE="verify" ;;
        --help|-h)
            echo "Usage: $0 [OPTION]"
            echo "  (no arg)   Full pipeline (fmt → fix → clippy → test → verify)"
            echo "  --check    fmt + check only"
            echo "  --clippy   Full lint cycle (fix → fmt → clippy)"
            echo "  --test     fmt + test + verify"
            echo "  --verify   Run ev check against sample fixtures"
            exit 0
            ;;
        *) echo "Unknown: $1"; echo "Usage: $0 [--check|--clippy|--test|--verify]"; exit 1 ;;
    esac
    shift
done

# ── Pre-flight auto-fixes ────────────────────────────────────────────────

run_fmt()          { cargo fmt --all; }
run_clippy_fix()   { cargo clippy --fix --allow-dirty 2>&1 || true; }
run_compiler_fix() { cargo fix --allow-dirty 2>&1 || true; }
run_auto_fix()     { run_fmt && run_clippy_fix && run_compiler_fix && run_fmt; }

# ── Strict checks ─────────────────────────────────────────────────────────

run_check()  { cargo check; }
run_clippy() { cargo clippy --all-targets -- -D warnings; }
run_test()   { cargo test --release; }

# ── Pipeline verification ─────────────────────────────────────────────────

ALL_PASS="tests/fixtures/all_pass.xif.yaml"
MIXED="tests/fixtures/sample.xif.yaml"

run_verify() {
    echo "--- all-pass fixture ---"
    echo "\$ ev check --target $ALL_PASS"
    cargo run --release -- check --target "$ALL_PASS"
    echo "  exit: 0 (expected)"
    echo ""

    echo "--- mixed pass/fail fixture (eq constraint) ---"
    echo "\$ ev check --target $MIXED"
    set +e
    cargo run --release -- check --target "$MIXED"
    EC=$?
    set -e
    if [ "$EC" -eq 1 ]; then
        echo "  exit: 1 (expected — 84 of 96 fail eq constraint)"
    else
        echo "  exit: $EC (UNEXPECTED)"
        exit 1
    fi
    echo ""

    echo "--- json output ---"
    echo "\$ ev check --target $MIXED --json | head -8"
    set +e
    cargo run --release -- check --target "$MIXED" --json 2>&1 | head -8
    set -e
    echo ""
}

# ── Full pipeline ─────────────────────────────────────────────────────────

run_all() {
    echo "=== Step 0: fmt --all ===" && run_fmt
    echo "=== Step 1: clippy --fix ===" && run_clippy_fix
    echo "=== Step 2: cargo fix ===" && run_compiler_fix
    echo "=== Step 3: fmt (after fixes) ===" && run_fmt
    echo "=== Step 4: cargo check ===" && run_check
    echo "=== Step 5: clippy ===" && run_clippy
    echo "=== Step 6: test ===" && run_test
    echo "=== Step 7: verify ===" && run_verify
}

# ── Dispatch ───────────────────────────────────────────────────────────────

case $MODE in
    check)
        echo "ev — check mode"
        run_fmt
        run_check
        echo ""
        echo "Check passed."
        ;;
    clippy)
        echo "ev — lint mode"
        run_auto_fix
        run_clippy
        echo ""
        echo "Lint passed."
        ;;
    test)
        echo "ev — test mode"
        run_fmt
        run_test
        run_verify
        echo ""
        echo "Tests passed."
        ;;
    verify)
        echo "ev — verify mode"
        run_verify
        echo ""
        echo "Verify passed."
        ;;
    all)
        echo "ev CI (local)"
        echo ""
        run_all
        echo ""
        echo "══════════════════════════════════════"
        echo "  All checks passed."
        echo "══════════════════════════════════════"
        ;;
esac
