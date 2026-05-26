#!/usr/bin/env bash
set -euo pipefail
#
# ev — Local CI runner
#
# Single entry point. Run without arguments for the full pipeline.
# Requires: yosys installed and ssccs available at ../ssccs for --demo.
#
# Usage:
#   ./run.sh              # Full pipeline (default)
#   ./run.sh --code       # fmt + clippy + build + test only
#   ./run.sh --fix        # fmt + clippy --fix + build + test (auto-fix)
#   ./run.sh --verify     # verify fixtures + Yosys synthesis
#   ./run.sh --demo       # channel demo: ev ↔ SSCCS POC golden anchors
#   ./run.sh --help       # show this message
#

cd "$(dirname "$0")"

case ${1:-} in
    --code)
        echo "══════════════════════════════════════"
        echo "  ev — code checks"
        echo "══════════════════════════════════════"
        export RUSTFLAGS="-D warnings"
        cargo fmt --check
        cargo clippy --all-targets
        cargo build --release
        cargo test --release
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
        export RUSTFLAGS="-D warnings"
        cargo build --release
        cargo test --release
        echo ""
        echo "  All checks passed (with auto-fix)."
        echo "══════════════════════════════════════"
        ;;
    --verify)
        echo "══════════════════════════════════════"
        echo "  ev — integration verification"
        echo "══════════════════════════════════════"
        if [ ! -f target/release/ev ]; then
            echo "  Binary not found. Run './run.sh' first to build."
            exit 1
        fi
        scripts/ci-integration.sh
        ;;
    --demo)
        exec bash scripts/demo-ssccs-poc.sh
        ;;
    --help|-h)
        echo "Usage: $0 [OPTION]"
        echo "  (no arg)   Full pipeline: fix → fmt → clippy → build → test → verify → demo"
        echo "  --code     fmt → clippy → build → test (strict, no auto-fix)"
        echo "  --fix      auto-fix → build → test (fast)"
        echo "  --verify   Yosys synthesis + fixtures only"
        echo "  --demo     Channel demo: ev ↔ SSCCS POC golden anchors"
        exit 0
        ;;
    *)
        # Full pipeline: fix, check, integration, demo
        echo "══════════════════════════════════════"
        echo "  ev — Full Pipeline"
        echo "══════════════════════════════════════"
        echo ""

        echo "=== Phase 1: auto-fix ==="
        cargo fmt --all
        cargo clippy --fix --allow-dirty 2>&1 || true
        cargo fix --allow-dirty 2>&1 || true
        cargo fmt --all
        echo ""

        echo "=== Phase 2: strict code checks ==="
        export RUSTFLAGS="-D warnings"
        cargo fmt --check
        cargo clippy --all-targets
        echo ""

        echo "=== Phase 3: build + test ==="
        cargo build --release
        cargo test --release
        echo ""

        echo "=== Phase 4: integration (Yosys + fixtures) ==="
        if command -v yosys &>/dev/null; then
            scripts/ci-integration.sh
        else
            echo "  yosys not found — skipping synthesis test"
            echo "  (install yosys: brew install yosys / apt install yosys)"
            # still run fixture-only checks
            EV=./target/release/ev
            ALL_PASS=tests/fixtures/all_pass.xif.yaml
            MIXED=tests/fixtures/sample.xif.yaml
            $EV check --target "$ALL_PASS"
            EC=0; $EV check --target "$MIXED" || EC=$?
            if [ "$EC" -eq 1 ]; then echo "  mixed: exit 1 (expected)"; fi
        fi
        echo ""

        echo "=== Phase 5: channel demo (ev ↔ SSCCS POC) ==="
        if [ -d "$(cd .. && pwd)/ssccs" ]; then
            # Resolve absolute path so demo script works regardless of its own cd
            SSCCS_DIR="$(cd .. && pwd)/ssccs"
            echo "  ssccs found at $SSCCS_DIR"
            set +e
            SSCCS_DIR="$SSCCS_DIR" bash scripts/demo-ssccs-poc.sh
            DEMO_EC=$?
            set -e
            if [ "$DEMO_EC" -ne 0 ]; then
                echo "  demo exited with code $DEMO_EC (non-fatal)"
            fi
        else
            echo "  ../ssccs not found — skipping demo"
            echo "  (clone: git clone https://github.com/ssccsorg/ssccs.git ../ssccs)"
        fi
        echo ""

        echo "══════════════════════════════════════"
        echo "  All done."
        echo "══════════════════════════════════════"
        ;;
esac
