#!/usr/bin/env bash
set -euo pipefail
#
# The demonstration, in the order it is recorded.
#
#   ./demo.sh            run it in a terminal
#   asciinema rec --title "smr_cases" -c "./demo.sh" demo.cast
#
# The four steps are the four claims: the declared space is classified, the same
# rules hold at a study scale, the record is reproducible, and the same points
# cost more in an indexed database.
cd "$(dirname "$0")"

# A recording reads better with a beat between the four screens.
PAUSE="${DEMO_PAUSE:-0}"
pause() {
    sleep "$PAUSE"
}

./run.sh
pause
echo
SPEC=study.xif.yaml ./run.sh
pause
echo
./run.sh --reproduce
pause
echo
python3 compare.py
pause
echo
cat <<'EOF'
what these four screens establish
  the admissible set follows from the declared rules, not from a hand-kept list
  the raw product is counted without enumeration, so cost follows the emitted set
  the record is addressed by spec_hash and reproduces byte for byte
  an axis subset reads from the key, so no index is stored

what they do not establish
  that the rules match a design basis; a wrong rule gives a confident wrong count
  that any case ran, or that it met an allowable
  that the comparison matches another workflow; one baseline, one machine
EOF
