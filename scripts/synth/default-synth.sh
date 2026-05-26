#!/usr/bin/env bash
set -euo pipefail
#
# default-synth.sh — Synthesis channel
#
# Takes a SystemVerilog file, runs Yosys synthesis, and produces a
# machine-readable JSON report containing module name, gate count,
# cell area, and DOT gate-level diagram path.
#
# Usage:
#   ./scripts/synth/default-synth.sh <file.sv> [top_module]
#
# If top_module is omitted, the script infers it from the filename
# (the basename without extension).
#
# Output: JSON to stdout
#
# Requires: yosys (https://github.com/YosysHQ/yosys)

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

# ── Argument parsing ──────────────────────────────────────────────────────

if [ $# -lt 1 ]; then
    cat <<USAGE >&2
Usage: $0 <file.sv> [top_module]

Synthesize a SystemVerilog design with Yosys and produce a JSON report
including gate-level DOT diagram path.

Arguments:
  file.sv       Path to the SystemVerilog source file
  top_module    (Optional) Top-level module name; defaults to filename stem

Output:
  JSON report to stdout with keys:
    tool, version, source, module_name, gate_count, cell_area,
    netlist_path, dot_path, status
USAGE
    exit 1
fi

SV_FILE="$1"
TOP_MODULE="${2:-}"

if [ ! -f "$SV_FILE" ]; then
    echo "{\"tool\":\"yosys\",\"status\":\"error\",\"message\":\"File not found: $SV_FILE\"}"
    exit 1
fi

if [ -z "$TOP_MODULE" ]; then
    BASENAME="$(basename "$SV_FILE")"
    BASENAME="${BASENAME%.sv}"
    BASENAME="${BASENAME%.v}"
    TOP_MODULE="$BASENAME"
fi

# ── Prerequisite check ───────────────────────────────────────────────────

if ! command -v yosys &>/dev/null; then
    cat <<MSG >&2
Error: yosys not found.

Install Yosys:
  macOS: brew install yosys
  Linux: see https://github.com/YosysHQ/yosys#building-from-source

MSG
    echo "{\"tool\":\"yosys\",\"status\":\"error\",\"message\":\"yosys not found in PATH\"}"
    exit 1
fi

YOSYS_VERSION="$(yosys --version 2>&1 | head -1)"

# ── Synthesis script ──────────────────────────────────────────────────────
#
# Flow:
#   1. Read the SystemVerilog source
#   2. Select the top module
#   3. Synthesize down to generic gates (synth -top)
#   4. Export gate-level DOT diagram (show -format dot)
#   5. Print statistics (stat -json)
#   6. Write JSON netlist (write_json) for potential post-processing

WORK_DIR="$(mktemp -d 2>/dev/null || mktemp -d -t 'yosys-synth')"
trap 'rm -rf "$WORK_DIR"' EXIT

JSON_NETLIST="$WORK_DIR/netlist.json"
DOT_FILE="$WORK_DIR/netlist.dot"
STAT_LOG="$WORK_DIR/stat.log"

yosys -q -l "$WORK_DIR/yosys.log" -p "
    read_verilog -sv \"$SV_FILE\";
    synth -top \"$TOP_MODULE\";
    show -format dot -prefix \"$WORK_DIR/netlist\" \"$TOP_MODULE\";
    stat -json > \"$STAT_LOG\";
    write_json \"$JSON_NETLIST\";
" 2>&1

YOSYS_EXIT=$?

if [ $YOSYS_EXIT -ne 0 ]; then
    echo "{\"tool\":\"yosys\",\"version\":\"$YOSYS_VERSION\",\"status\":\"error\",\"message\":\"Yosys exited with code $YOSYS_EXIT\",\"source\":\"$SV_FILE\",\"module_name\":\"$TOP_MODULE\"}"
    exit 1
fi

# ── Parse stat JSON ──────────────────────────────────────────────────────

GATE_COUNT="null"
CELL_AREA="null"

if [ -f "$STAT_LOG" ]; then
    GATE_COUNT=$(python3 -c "
import json, sys
try:
    with open('$STAT_LOG') as f:
        data = json.load(f)
    mod = data.get('top_module', data.get('design', {}))
    nc = mod.get('num_cells')
    print(nc if nc is not None else 'null')
except Exception:
    print('null')
" 2>/dev/null || echo "null")

    CELL_AREA=$(python3 -c "
import json, sys
try:
    with open('$STAT_LOG') as f:
        data = json.load(f)
    mod = data.get('top_module', data.get('design', {}))
    area = mod.get('area')
    print(area if area is not None else 'null')
except Exception:
    print('null')
" 2>/dev/null || echo "null")
fi

# Verify DOT file was created
DOT_PATH_JSON="null"
if [ -f "$DOT_FILE" ]; then
    DOT_PATH_JSON="\"$DOT_FILE\""
fi

# ── Output JSON report ───────────────────────────────────────────────────

python3 -c "
import json

report = {
    'tool': 'yosys',
    'version': '$YOSYS_VERSION',
    'source': '$SV_FILE',
    'module_name': '$TOP_MODULE',
    'gate_count': $GATE_COUNT,
    'cell_area': $CELL_AREA,
    'netlist_path': '$JSON_NETLIST',
    'dot_path': $DOT_PATH_JSON,
    'status': 'ok',
}

print(json.dumps(report, indent=2))
"
