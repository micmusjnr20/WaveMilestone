#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd")

# ── Build first, then deploy to testnet ───────────────────
echo "==> Step 1: Build release WASM..."
"$SCRIPT_DIR/build.sh" release

# ── Deploy to testnet ─────────────────────────────────────
echo "==> Step 2: Deploy to testnet..."
"$SCRIPT_DIR/deploy.sh" testnet

echo "==> WaveMilestone deployed to Stellar testnet."
echo "    Contract ID: $(cat .contract-id 2>/dev/null || echo 'unknown')"
echo ""
echo "    Next steps:"
echo "      1. Deploy WaveGuard and note its contract ID."
echo "      2. Call create_milestone_pool with the WaveGuard address,"
echo "         asset contract ID, and total funds."
echo "      3. Use release_issue_bounty to pay out contributors."
echo ""
