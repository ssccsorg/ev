#!/usr/bin/env bash
set -euo pipefail
#
# Design-basis case space: classify the declared space and print its record.
#
#   ./run.sh              summary in words
#   ./run.sh --raw        the engine's own text output
#   ./run.sh --json       the Fact envelope on stdout
#   ./run.sh --record     the stable payload on stdout
#   ./run.sh --reproduce  run twice, compare the payloads, show the stamps move
#
# SPEC selects the spec, default space.xif.yaml. SPEC=study.xif.yaml ./run.sh
# runs the larger design-of-experiments space.
#
# The record carries the summary and the spec address. The per-point rows are
# opt-in (`--rows`), because they cost about 400 bytes a point: the study space
# writes 675 bytes without them and 317 MB with them.
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
        jq -r '.payload | @base64d'
    else
        python3 -c 'import base64, json, sys; sys.stdout.write(base64.b64decode(json.load(sys.stdin)["payload"]).decode())'
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

# One text run, then the fields the summary needs.
summary() {
    local status=0
    local text
    text=$("$EV" verify --target "$SPEC" 2>&1) || status=$?
    if [ "$status" -gt 1 ]; then
        printf '%s\n' "$text" >&2
        exit "$status"
    fi
    STATUS="$status"
    TARGET=$(printf '%s\n' "$text" | sed -n 's/^target: *//p')
    ENGINE=$(printf '%s\n' "$text" | sed -n 's/^engine: *//p')
    CLAIM=$(printf '%s\n' "$text" | sed -n 's/^claim: *//p')
    TOTAL=$(printf '%s\n' "$text" | sed -n 's/^total: *//p')
    PASSED=$(printf '%s\n' "$text" | sed -n 's/^passed: *//p')
    FAILED=$(printf '%s\n' "$text" | sed -n 's/^failed: *//p')
}

thousands() {
    awk -v n="$1" 'BEGIN {
        s = sprintf("%d", n); out = "";
        while (length(s) > 3) { out = "," substr(s, length(s) - 2) out; s = substr(s, 1, length(s) - 3) }
        print s out
    }'
}

stamp() {
    printf '%s' "$1" | tr -d '\n' | grep -o -m1 '"timestamp"[^,]*' | sed 's/.*:[[:space:]]*"//; s/"$//' || true
}

spec_hash() {
    local record
    record=$(fact | decode_payload)
    printf '%s' "$record" | tr -d '\n' | grep -o -m1 '"spec_hash":"[^"]*"' | sed 's/.*:"//; s/"$//'
}

case "${1:-}" in
    --raw)
        status=0
        "$EV" verify --target "$SPEC" || status=$?
        printf '\nexit %s: the space contains rejected combinations\n' "$status"
        ;;
    --json)
        fact
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
        summary
        HASH=$(spec_hash)
        printf '\n%s\n\n' "$TARGET"
        printf '  spec       %s\n' "$SPEC"
        printf '  spec_hash  %s\n' "$HASH"
        printf '  engine     %s\n' "$ENGINE"
        printf '  claim      %s\n\n' "$CLAIM"
        printf '  declared    %12s   combinations the spec declares\n' "$(thousands "$TOTAL")"
        printf '  admissible  %12s   combinations the rules admit, emitted and classified\n' "$(thousands "$PASSED")"
        printf '  rejected    %12s   combinations the rules exclude, never generated\n\n' "$(thousands "$FAILED")"
        printf '  exit        %12s   1 means the space has rejected combinations\n\n' "$STATUS"
        printf 'The admissible set is the case list. No allowable is evaluated.\n'
        ;;
esac
