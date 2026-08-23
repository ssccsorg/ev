#!/usr/bin/env bash
set -euo pipefail
#
# coverage.sh: ev code coverage gate (cargo-llvm-cov).
#
# Runs the instrumented test suite, then exercises the external-tool
# backends (Spike, Yosys, simulation) with the instrumented binary so their
# code is covered, then merges everything and enforces the 80% lines and
# 80% regions thresholds. The backends run with local tools when present
# and inside the ev Docker image otherwise.
#
# The two-step llvm-cov flow: `cargo llvm-cov --no-report` leaves the
# profraw files in target/llvm-cov-target/, the external runs append their
# profraw to the same directory, and `cargo llvm-cov report` merges all of
# them. The merge glob is target/llvm-cov-target/*.profraw, so external
# runs must write there (LLVM_PROFILE_FILE below).
#
# The whole instrumented build directory is removed before the suite. A
# stale incremental build after a source change leaks old binary signatures
# into the merge, producing mismatched-data warnings and distorted coverage
# (observed after the PR #49 conflict resolution, which dropped the report
# to ~51% before a full clean restored it).
#
# Usage: bash scripts/coverage.sh
#

cd "$(dirname "$0")/.."

if ! command -v cargo-llvm-cov >/dev/null 2>&1; then
    echo "ERROR: cargo-llvm-cov not found. Install it with:"
    echo "  rustup component add llvm-tools-preview"
    echo "  cargo install cargo-llvm-cov --locked"
    exit 1
fi

EV_IMAGE="${EV_IMAGE:-ghcr.io/ssccsorg/ev:latest}"
EV_COV=./target/llvm-cov-target/release/ev
ALL_PASS=tests/fixtures/common/all_pass.xif.yaml
PROFRAW_DIR=target/llvm-cov-target
COVERAGE_MIN="${COVERAGE_MIN:-80}"

echo "=== coverage: instrumented test suite ==="
rm -rf "$PROFRAW_DIR"
cargo llvm-cov --release --no-report

echo "=== coverage: external backend runs (instrumented binary) ==="

# Mock simulation backend, no external tools.
echo "--- mock simulation ---"
EV_SIM_BACKEND=mock LLVM_PROFILE_FILE="$PROFRAW_DIR/ev-ext-%p-%m.profraw" \
    "$EV_COV" simulate --target "$ALL_PASS" >/dev/null

# Yosys synthesis backend, local yosys or the ev image.
echo "--- yosys synthesis ---"
if command -v yosys >/dev/null 2>&1; then
    EV_SYNTH_BACKEND=yosys LLVM_PROFILE_FILE="$PROFRAW_DIR/ev-ext-%p-%m.profraw" \
        "$EV_COV" synth --target "$ALL_PASS" >/dev/null
else
    docker run --rm --pull=always -v "$(pwd):/workspace" -w /workspace \
        -e EV_SYNTH_BACKEND=yosys \
        -e LLVM_PROFILE_FILE="/workspace/$PROFRAW_DIR/ev-ext-%p-%m.profraw" \
        "$EV_IMAGE" bash -c "cd /workspace && EV_SYNTH_BACKEND=yosys ./target/llvm-cov-target/release/ev synth --target tests/fixtures/common/all_pass.xif.yaml" >/dev/null
fi

# Spike simulation backend, local spike + pk + riscv gcc or the ev image.
echo "--- spike simulation ---"
if command -v spike >/dev/null 2>&1 && command -v riscv64-unknown-elf-gcc >/dev/null 2>&1; then
    EV_SIM_BACKEND=spike EV_PK_PATH="${EV_PK_PATH:-pk}" \
        LLVM_PROFILE_FILE="$PROFRAW_DIR/ev-ext-%p-%m.profraw" \
        "$EV_COV" simulate --target "$ALL_PASS" >/dev/null
else
    docker run --rm --pull=always -v "$(pwd):/workspace" -w /workspace \
        -e EV_SIM_BACKEND=spike \
        -e EV_PK_PATH=/usr/local/riscv64-unknown-elf/bin/pk \
        -e LLVM_PROFILE_FILE="/workspace/$PROFRAW_DIR/ev-ext-%p-%m.profraw" \
        "$EV_IMAGE" bash -c "cd /workspace && EV_SIM_BACKEND=spike EV_PK_PATH=/usr/local/riscv64-unknown-elf/bin/pk ./target/llvm-cov-target/release/ev simulate --target tests/fixtures/common/all_pass.xif.yaml" >/dev/null
fi

echo "=== coverage: merged report and thresholds ==="
cargo llvm-cov report --release --fail-under-lines "$COVERAGE_MIN" --fail-under-regions "$COVERAGE_MIN"
echo "fixture coverage: tagma decoder 11,172/11,172, tagma demo top 11,172/11,172, cva6 ref 196,608, ibex rv32imcb 92,160"
