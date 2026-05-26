#!/usr/bin/env bash
set -euo pipefail
#
# CI — build, lint, unit-test phase
#
# Separated from integration tests so CI can fail fast on code issues
# before spending time on heavy integration steps.
#
# Usage:
#   ./scripts/ci.sh               # strict CI mode (fmt, clippy, build, test)
#   ./scripts/ci.sh --fix          # auto-fix mode (fmt, clippy --fix, test)
#

cd "$(dirname "$0")/.."

FIX=false
if [[ "${1:-}" == "--fix" ]]; then
    FIX=true
fi

if $FIX; then
    cargo fmt --all
    cargo clippy --fix --allow-dirty 2>&1 || true
    cargo fix --allow-dirty 2>&1 || true
    cargo fmt --all
fi

echo "=== fmt ==="
cargo fmt --check

echo "=== clippy ==="
cargo clippy --all-targets -- -D warnings

echo "=== build ==="
cargo build --release

echo "=== test ==="
cargo test --release
