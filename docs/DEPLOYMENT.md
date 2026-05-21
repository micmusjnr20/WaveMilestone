# WaveMilestone Deployment Guide

## Prerequisites

- [Soroban CLI](https://soroban.stellar.org/docs/getting-started/setup) installed
- Rust `wasm32-unknown-unknown` target installed (`rustup target add wasm32-unknown-unknown`)
- Stellar account with testnet/mainnet funds for deployment fees
- Deployed [WaveGuard](https://github.com/anomalyco/waveguard) contract instance
- Deployed Stellar Asset Contract (SAC) for the payout token

## Deployment Steps

### 1. Build the Contract

```bash
# Build optimized WASM
./scripts/build.sh release
```

The optimized WASM will be at:
`target/wasm32-unknown-unknown/release/wave_milestone_optimized.wasm`

### 2. Configure Environment

```bash
cp .env.example .env
```

Edit `.env` with your Stellar credentials:

```env
STELLAR_NETWORK_PASSPHRASE="Test SDF Network ; September 2015"
STELLAR_RPC_URL="https://soroban-testnet.stellar.org"
DEPLOYER_SECRET_KEY="<your-secret-key>"
```

### 3. Deploy to Testnet

```bash
# One-command build + deploy
./scripts/deploy_testnet.sh

# Or step by step:
./scripts/build.sh release
./scripts/deploy.sh testnet
```

Save the returned **Contract ID** — you'll need it for all interactions.

### 4. Verify Deployment

```bash
soroban contract read \
    --id <CONTRACT_ID> \
    --rpc-url https://soroban-testnet.stellar.org \
    --network-passphrase "Test SDF Network ; September 2015"
```

### 5. Initialize the Contract

Call `create_milestone_pool` to set up the first milestone:

```bash
soroban contract invoke \
    --id <CONTRACT_ID> \
    --source <MAINTAINER_KEY> \
    --rpc-url https://soroban-testnet.stellar.org \
    --network-passphrase "Test SDF Network ; September 2015" \
    -- \
    create_milestone_pool \
    --maintainer <MAINTAINER_ADDRESS> \
    --guard_contract <WAVEGUARD_ID> \
    --asset <SAC_TOKEN_ID> \
    --total_funds <AMOUNT> \
    --expiry <UNIX_TIMESTAMP>
```

### 6. Release Bounties

```bash
soroban contract invoke \
    --id <CONTRACT_ID> \
    --source <MAINTAINER_KEY> \
    --rpc-url https://soroban-testnet.stellar.org \
    --network-passphrase "Test SDF Network ; September 2015" \
    -- \
    release_issue_bounty \
    --maintainer <MAINTAINER_ADDRESS> \
    --repo_hash <32_BYTE_HEX> \
    --issue_id <NUMBER> \
    --developer <CONTRIBUTOR_ADDRESS> \
    --amount <PAYOUT_AMOUNT>
```

### 7. Clawback Unclaimed Funds (After Expiry)

```bash
soroban contract invoke \
    --id <CONTRACT_ID> \
    --source <MAINTAINER_KEY> \
    --rpc-url https://soroban-testnet.stellar.org \
    --network-passphrase "Test SDF Network ; September 2015" \
    -- \
    clawback_expired_funds \
    --maintainer <MAINTAINER_ADDRESS>
```

## Mainnet Deployment

Repeat the same steps with mainnet configuration:

```env
STELLAR_NETWORK_PASSPHRASE="Public Global Stellar Network ; September 2015"
STELLAR_RPC_URL="https://soroban.stellar.org"
```

**IMPORTANT**: Test thoroughly on testnet before deploying to mainnet. Verify all edge cases with real token amounts on testnet first.

## Post-Deployment Checklist

- [ ] Contract deployed and verified
- [ ] WaveGuard contract deployed and maintainers registered
- [ ] Asset contract (SAC) deployed and funded
- [ ] `create_milestone_pool` called successfully
- [ ] `release_issue_bounty` tested with a test issue
- [ ] `clawback_expired_funds` tested (after fast-forwarding on testnet)
- [ ] All view methods return expected data
- [ ] Contract ID recorded in `.contract-id`

## Network-Specific Notes

### Testnet
- No real value involved — safe for experimentation.
- Use the Stellar Friendbot to fund deployer account: `soroban config identity fund <IDENTITY>`
- RPC rate limits are generous but not unlimited.

### Mainnet
- Real assets at stake. Audit contract and test thoroughly first.
- Consider a time-locked multisig for the maintainer key.
- Monitor RPC costs: each `release_issue_bounty` call costs a small fee in XLM.
- Set `expiry` far enough in the future to cover the milestone timeline.

## Upgradeability

WaveMilestone uses an **immutable contract pattern**. Once deployed, the contract logic cannot be upgraded. To migrate:

1. Deploy a new WaveMilestone instance.
2. Call `clawback_expired_funds` on the old instance (once expired).
3. Fund the new instance and resume operations.

This guarantees that users' understanding of the contract behavior remains correct for the lifetime of a milestone.
