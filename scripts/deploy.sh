#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_ROOT"

# ── Load environment ───────────────────────────────────────
if [ -f .env ]; then
    set -a
    source .env
    set +a
elif [ -f .env.example ]; then
    echo "==> No .env found. Copy .env.example to .env and fill in values."
    exit 1
fi

# ── Required variables ────────────────────────────────────
: "${DEPLOYER_SECRET_KEY:?DEPLOYER_SECRET_KEY not set}"
: "${STELLAR_RPC_URL:?STELLAR_RPC_URL not set}"
: "${STELLAR_NETWORK_PASSPHRASE:?STELLAR_NETWORK_PASSPHRASE not set}"

NETWORK="${1:-testnet}"
WASM_FILE="${2:-target/wasm32-unknown-unknown/release/wave_milestone_optimized.wasm}"
DEPLOY_SALT="${DEPLOY_SALT:-}"

if [ ! -f "$WASM_FILE" ]; then
    echo "==> WASM file not found: $WASM_FILE"
    echo "    Run ./scripts/build.sh first."
    exit 1
fi

# ── Deploy ─────────────────────────────────────────────────
echo "==> Deploying WaveMilestone to $NETWORK..."
echo "    RPC:   $STELLAR_RPC_URL"
echo "    WASM:  $WASM_FILE"

DEPLOY_CMD="soroban contract deploy \
    --wasm \"$WASM_FILE\" \
    --source \"$DEPLOYER_SECRET_KEY\" \
    --rpc-url \"$STELLAR_RPC_URL\" \
    --network-passphrase \"$STELLAR_NETWORK_PASSPHRASE\""

if [ -n "$DEPLOY_SALT" ]; then
    DEPLOY_CMD="$DEPLOY_CMD --salt \"$DEPLOY_SALT\""
fi

echo "==> Running: soroban contract deploy ..."
CONTRACT_ID=$(eval "$DEPLOY_CMD")

echo "==> Deployed!"
echo "    Contract ID: $CONTRACT_ID"

# ── Save contract ID ──────────────────────────────────────
echo "$CONTRACT_ID" > .contract-id
echo "==> Contract ID saved to .contract-id"

# ── Verify deployment ─────────────────────────────────────
echo "==> Verifying deployment..."
soroban contract read \
    --id "$CONTRACT_ID" \
    --rpc-url "$STELLAR_RPC_URL" \
    --network-passphrase "$STELLAR_NETWORK_PASSPHRASE" \
    --source "$DEPLOYER_SECRET_KEY" 2>/dev/null || echo "    (Contract deployed; no state read until initialized)"

echo "==> Done."
