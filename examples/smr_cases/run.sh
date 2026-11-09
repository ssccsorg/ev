#!/usr/bin/env bash
set -euo pipefail
#
# Design-basis case space: classify the declared space and print its record.
#
#   ./run.sh              summary and record identity
#   ./run.sh --json       the Fact envelope on stdout
#   ./run.sh --record     the stable payload on stdout
#   ./run.sh --reproduce  run twice, compare the payloads, show the stamps move
#
# SPEC selects the spec, default space.xif.yaml. SPEC=study.xif.yaml ./run.sh
# runs the larger design-of-experiments space.
#
# `ev verify` exits 1 when the declared space contains a combination the rules
# reject. That is a verdict on the space, not a failure of the run, so the
# summary prints it as a value.
#
# The Fact envelope carries a timestamp, so two runs differ there. The payload
# is the record and is identical across runs; --reproduce checks exactly that.
#
cd "$(dirname "$0")"
SPEC="${SPEC:-space.xif.yaml}"
EV="${EV:-$(cd ../.. && pwd)/target/release/ev}"

if [ ! -x "$EV" ]; then
    ( cd ../.. && cargo build --release )
fi

# Reads a Fact envelope on stdin and writes the payload JSON on stdout.
decode_payload() {
    if command -v jq >/dev/null 2>&1; then
        jq -r '.payload | implode'
    else
        python3 -c 'import json, sys; sys.stdout.write(bytes(json.load(sys.stdin)["payload"]).decode())'
    fi
}

fact() {
    local status=0
    "$EV" verify --target "$SPEC" --format json || status=$?
    if [ "$status" -gt 1 ]; then
        echo "ev verify exited $status" >&2
        exit "$status"
    fi
}

stamp() {
    printf '%s' "$1" | tr -d '\n' | grep -o -m1 '"timestamp"[^,]*' | sed 's/.*:[[:space:]]*"//; s/"$//' || true
}

case "${1:-}" in
    --json)
        exec "$EV" verify --target "$SPEC" --format json
        ;;
    --record)
        fact | decode_payload
        ;;
    --reproduce)
        first=$(fact)
        second=$(fact)
        payload_first=$(printf '%s' "$first" | decode_payload)
        payload_second=$(printf '%s' "$second" | decode_payload)
        hash_first=$(printf '%s' "$payload_first" | shasum -a 256 | cut -d' ' -f1)
        hash_second=$(printf '%s' "$payload_second" | shasum -a 256 | cut -d' ' -f1)
        printf 'payload hash, run 1: %s\n' "$hash_first"
        printf 'payload hash, run 2: %s\n' "$hash_second"
        printf 'envelope stamp, run 1: %s\n' "$(stamp "$first")"
        printf 'envelope stamp, run 2: %s\n' "$(stamp "$second")"
        if [ "$hash_first" = "$hash_second" ]; then
            echo 'payloads identical'
        else
            echo 'payloads differ'
            exit 1
        fi
        ;;
    *)
        status=0
        "$EV" verify --target "$SPEC" || status=$?
        printf 'exit:   %s (1 means the space contains a rejected combination)\n' "$status"
        record=$(fact | decode_payload)
        spec_hash=$(printf '%s' "$record" | tr -d '\n' | grep -o -m1 '"spec_hash"[^,]*' | sed 's/.*"\([^"]*\)"$/\1/')
        points=$(printf '%s' "$record" | grep -o '"point_id"' | wc -l | tr -d ' ')
        printf '\nrecord\n  spec_hash: %s\n  points:    %s\n' "$spec_hash" "$points"
        ;;
esac
