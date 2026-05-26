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

grep -q '"fact_type":"synthesis_result"' /tmp/synth_fact.json || (echo "FAILED: missing fact_type"; exit 1)
grep -q '"status":"ok"' /tmp/synth_fact.json || (echo "FAILED: synthesis status not ok"; exit 1)
echo "synthesis Fact OK"

# ── Fixture verification ────────────────────────────────────────────

echo "=== all-pass fixture ==="
$EV check --target "$ALL_PASS"

echo "=== mixed fixture (exit 1 expected) ==="
$EV check --target "$MIXED" && exit 1 || true
echo "  exit 1: correct (84 of 96 fail eq constraint)"

echo "=== json output ==="
$EV check --target "$MIXED" --json | head -8

# ── Channel demo (ev ↔ SSCCS POC) ───────────────────────────────────

echo "=== channel demo (ev ↔ SSCCS POC) ==="
SSCCS_DIR=../ssccs bash scripts/demo-ssccs-poc.sh
