#!/usr/bin/env bash
# Local CI: format, lint and test gates. Run before every commit.
# Prints only failures and a final OK line; exit code is the gate.
set -euo pipefail
cd "$(dirname "$0")/.."
log=$(mktemp)
trap 'rm -f "$log"' EXIT
run() {
    if ! "$@" >"$log" 2>&1; then
        echo "FAILED: $*"
        grep -E "^(error|warning)|-->|FAILED|panicked|Diff in" -A8 "$log" | head -80
        exit 1
    fi
}
run cargo fmt --check
run cargo clippy --all-targets -- -D warnings
run cargo test --quiet
echo "CI OK: $(grep -Eo '[0-9]+ passed' "$log" | awk '{s+=$1} END {print s}') tests passed"
