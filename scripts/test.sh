#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$(dirname "$SCRIPT_DIR")"

export RUST_LOG="${RUST_LOG:-info}"
export RUST_BACKTRACE="${RUST_BACKTRACE:-1}"

TEST_FILTER="${1:-}"

if [ -n "$TEST_FILTER" ]; then
    cargo test --workspace --no-fail-fast -- "$TEST_FILTER" --nocapture
else
    cargo test --workspace --no-fail-fast -- --nocapture
fi
