#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."
cargo build -p d2s --examples

temp_log="$(mktemp)"
subscriber_pid=""
cleanup() {
  if [[ -n "$subscriber_pid" ]]; then
    kill "$subscriber_pid" 2>/dev/null || true
    wait "$subscriber_pid" 2>/dev/null || true
  fi
  rm -f "$temp_log"
}
trap cleanup EXIT

# DomainId 10; three samples.
timeout 25s target/debug/examples/subscriber 10 3 >"$temp_log" 2>&1 &
subscriber_pid=$!
sleep 1

timeout 25s target/debug/examples/publisher 10 3
wait "$subscriber_pid"
subscriber_pid=""

echo "---- Subscriber output ----"
cat "$temp_log"
[[ "$(grep -c 'RECEIVED \[' "$temp_log")" -eq 3 ]]
echo "D2S Phase 0: PASS (three samples received)"
