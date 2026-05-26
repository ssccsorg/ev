#!/usr/bin/env bash
set -euo pipefail
#
# CI — integration test phase
#
# Requires: cargo build --release already run (binary at target/release/ev)
#
# Usage:
#   ./scripts/ci-integration.sh
#

cd "$(dirname "$0")/.."

EV=./target/release/ev
ALL_PASS=tests/fixtures/all_pass.xif.yaml
MIXED=tests/fixtures/sample.xif.yaml

# ── Yosys synthesis ─────────────────────────────────────────────────

echo "=== Yosys version ==="
yosys --version

echo "=== text output ==="
$EV check --target "$ALL_PASS" --synth

echo "=== json output ==="
$EV check --target "$ALL_PASS" --synth --json > /tmp/synth_fact.json 2>/tmp/synth_stderr.txt

if ! grep -q '"fact_type": "synthesis_result"' /tmp/synth_fact.json; then
    cat /tmp/synth_stderr.txt
    echo "FAILED: missing fact_type"
    exit 1
fi
if ! grep -q '"status": "ok"' /tmp/synth_fact.json; then
    cat /tmp/synth_stderr.txt
    echo "FAILED: synthesis status not ok"
    exit 1
fi
echo "synthesis Fact OK"

# ── Fixture verification ────────────────────────────────────────────

echo "=== all-pass fixture ==="
$EV check --target "$ALL_PASS"

echo "=== mixed fixture (exit 1 expected) ==="
EC=0
$EV check --target "$MIXED" || EC=$?
if [ "$EC" -eq 1 ]; then
    echo "  exit: 1 (correct — 84 of 96 fail eq constraint)"
else
    echo "  exit: $EC (UNEXPECTED)"
    exit 1
fi

echo "=== json output ==="
$EV check --target "$MIXED" --json | head -8

# ── Channel demo (ev ↔ SSCCS POC) ───────────────────────────────────
#
# Only runs in CI (where ../ssccs is set up by the workflow).
# Local runs should use ./run.sh --demo for a self-contained demo.

if [ -n "${CI:-}" ]; then
    echo "=== channel demo (ev ↔ SSCCS POC) ==="
    SSCCS_DIR=../ssccs bash scripts/demo-ssccs-poc.sh
else
    echo "=== channel demo: skipped (not in CI) ==="
    echo "  run './run.sh --demo' locally"
fi
