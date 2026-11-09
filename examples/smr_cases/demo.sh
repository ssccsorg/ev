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

./run.sh
echo
SPEC=study.xif.yaml ./run.sh
echo
./run.sh --reproduce
echo
python3 compare.py
