#!/usr/bin/env bash
set -euo pipefail
#
# ev — Local CI runner
#
# Delegates to scripts/ci.sh and scripts/ci-integration.sh.
#
# Usage:
#   ./run.sh              # Full pipeline: fmt + clippy --fix + test
#   ./run.sh --verify     # verify fixtures + Yosys synthesis
#   ./run.sh --demo       # channel demo: ev ↔ SSCCS POC golden anchors
#   ./run.sh --help       # show this message
#

cd "$(dirname "$0")"

case ${1:-} in
    --verify)
        if [ ! -f target/release/ev ]; then
            echo "Binary not found. Run './run.sh' first to build."
            exit 1
        fi
        scripts/ci-integration.sh ;;
    --demo)
        exec bash scripts/demo-ssccs-poc.sh ;;
    --help|-h)
        echo "Usage: $0 [OPTION]"
        echo "  (no arg)   Full pipeline: fmt → clippy --fix → test"
        echo "  --verify   Verify fixtures + Yosys synthesis"
        echo "  --demo     Channel demo: ev ↔ SSCCS POC golden anchors"
        exit 0
        ;;
    *)
        scripts/ci.sh --fix ;;
esac
