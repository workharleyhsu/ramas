#!/usr/bin/env bash
# Local CI: format, lint and test gates. Run before every commit.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --quiet
