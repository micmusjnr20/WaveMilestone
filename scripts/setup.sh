#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_ROOT"

echo "==> WaveMilestone Project Setup"
echo ""

# ── Check prerequisites ───────────────────────────────────
echo "==> Checking prerequisites..."

check_cmd() {
    if command -v "$1" &>/dev/null; then
        echo "  [OK] $1: $(command -v "$1")"
        return 0
    else
        echo "  [MISSING] $1"
        return 1
    fi
}

MISSING=0
check_cmd rustc || MISSING=1
check_cmd cargo || MISSING=1
check_cmd soroban || MISSING=1

if [ $MISSING -eq 1 ]; then
    echo ""
    echo "==> Missing required tools. Install them:"
    echo "    - Rust:   https://rustup.rs"
    echo "    - Soroban CLI: https://soroban.stellar.org/docs/getting-started/setup"
    exit 1
fi

# ── Toolchain ─────────────────────────────────────────────
echo ""
echo "==> Installing Rust WASM target..."
rustup target add wasm32-unknown-unknown

# ── Environment ───────────────────────────────────────────
echo ""
if [ ! -f .env ]; then
    echo "==> Creating .env from .env.example..."
    cp .env.example .env
    echo "    Edit .env with your Stellar credentials before deploying."
else
    echo "==> .env already exists; skipping."
fi

# ── Pre-commit hooks ──────────────────────────────────────
echo ""
if command -v pre-commit &>/dev/null; then
    echo "==> Installing pre-commit hooks..."
    pre-commit install --hook-type pre-commit --hook-type pre-push
else
    echo "==> pre-commit not found. Install it: pip install pre-commit"
fi

# ── Verify build ──────────────────────────────────────────
echo ""
echo "==> Verifying project builds..."
cargo check --workspace

echo ""
echo "==> Setup complete!"
echo ""
echo "    Next steps:"
echo "      1. Review .env and fill in your Stellar credentials."
echo "      2. Run ./scripts/test.sh to run the test suite."
echo "      3. Run ./scripts/build.sh release to build the WASM contract."
echo "      4. Deploy with ./scripts/deploy_testnet.sh"
