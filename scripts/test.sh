#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_ROOT"

# ── Configuration ──────────────────────────────────────────
RUST_LOG="${RUST_LOG:-info}"
RUST_BACKTRACE="${RUST_BACKTRACE:-1}"
TEST_FILTER="${1:-}"

export RUST_LOG
export RUST_BACKTRACE

# ── Build all targets (required for integration tests) ────
echo "==> Building all targets (including tests)..."
cargo build --workspace --all-targets

# ── Run tests ─────────────────────────────────────────────
echo "==> Running tests..."

if [ -n "$TEST_FILTER" ]; then
    echo "    Filter: $TEST_FILTER"
    cargo test --workspace -- "$TEST_FILTER" --nocapture
else
    cargo test --workspace -- --nocapture
fi

echo "==> All tests passed."
