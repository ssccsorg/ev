#!/usr/bin/env bash
set -euo pipefail
#
# The demonstration. Running it refreshes demo.cast next to this script.
#
#   ./demo.sh              record and play, about 30 s
#   ./demo.sh --play       play without recording
#
# DEMO_PAUSE sets the beat between screens, 3 s by default when recording.
# Inside a recording (ASCIINEMA_REC set) the script plays rather than records,
# so there is no recursion.
cd "$(dirname "$0")"

if [ "${1:-}" != "--play" ] && [ -z "${ASCIINEMA_REC:-}" ]; then
    if command -v asciinema >/dev/null 2>&1; then
        exec asciinema rec --overwrite --title smr_cases \
            -c "DEMO_PAUSE=${DEMO_PAUSE:-3} ./demo.sh --play" demo.cast
    fi
    echo "asciinema not found: playing without recording" >&2
fi

# A recording reads better with a beat between the screens.
PAUSE="${DEMO_PAUSE:-0}"
pause() {
    sleep "$PAUSE"
}

printf 'smr_cases: exhaustive verification by coordinate\n\n'

./run.sh
pause
echo
SPEC=study.xif.yaml ./run.sh
pause
echo
./run.sh --reproduce
pause
echo

COMPARE=../../target/release/examples/store_compare
if [ ! -x "$COMPARE" ]; then
    ( cd ../.. && cargo build --release --example store_compare --features store-compare >/dev/null 2>&1 )
fi
"$COMPARE"
pause
echo

# The same spec read by both readers, as a table rather than raw timings.
EV="${EV:-../../target/release/ev}"
measure() { # engine -> "wall peak_bytes"
    local stats=/tmp/ev-demo-stats
    /usr/bin/time -l env EV_CLASSIFIER="$1" "$EV" verify --target study.xif.yaml >/dev/null 2>"$stats" || true
    local wall peak
    wall=$(awk '/ real/ {print $1; exit}' "$stats")
    peak=$(awk 'tolower($0) ~ /maximum resident set size/ {\
        if ($0 ~ /kbytes/) printf "%.0f", $NF * 1024; else printf "%.0f", $1; exit }' "$stats")
    if [ -z "$wall" ]; then
        TIMEFORMAT='%R'
        wall=$( { time env EV_CLASSIFIER="$1" "$EV" verify --target study.xif.yaml >/dev/null; } 2>&1 | tail -1 )
        peak=0
    fi
    echo "$wall ${peak:-0}"
}
read -r scan_wall scan_peak <<<"$(measure naive)"
read -r coord_wall coord_peak <<<"$(measure structural)"
printf 'the same space, both readers\n\n'
printf '  declared 1,440,000   admitted 828,000\n\n'
printf '  %-14s %9s %9s\n' reader wall peak
printf '  %-14s %9s %9s\n' scan "$(printf '%.2f s' "$scan_wall")" "$(awk -v b="$scan_peak" 'BEGIN {printf "%.0f MB", b / 1048576}')"
printf '  %-14s %9s %9s\n' coordinate "$(printf '%.2f s' "$coord_wall")" "$(awk -v b="$coord_peak" 'BEGIN {printf "%.0f MB", b / 1048576}')"
printf '\n  the scan pays the declared product; the coordinate pays the admitted set\n'
pause
echo

cat <<'EOF'
what these screens establish
  the admissible set follows from the declared rules, not from a hand-kept list
  the scan pays the declared product; the coordinate reader pays the admitted set
  the record is addressed by spec_hash and reproduces byte for byte
  an axis subset needs no stored index: the address carries the axes

what they do not establish
  that the rules match a design basis; a wrong rule gives a confident wrong count
  that any case ran, or that it met an allowable
  that the comparison matches another workflow; one baseline, one machine
EOF
