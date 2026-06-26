# WaveMilestone Architecture

## System Overview

WaveMilestone is a Stellar Soroban smart contract that implements an automated milestone escrow vault. It links a GitHub Milestone budget to on-chain micro-payouts that are released as issues are completed.

```
┌──────────────────────────────────────────────────────────┐
│                     Off-Chain                            │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐  │
│  │ GitHub       │  │ Maintainer  │  │ Contributor     │  │
│  │ Milestone    │  │ (Wallet)    │  │ (Wallet)        │  │
│  └──────┬───────┘  └──────┬──────┘  └────────┬────────┘  │
│         │                 │                   │           │
└─────────┼─────────────────┼───────────────────┼───────────┘
          │            TX    │                   │
          ▼                 ▼                    ▼
┌──────────────────────────────────────────────────────────┐
│                     Stellar Network                       │
│  ┌──────────────────────────────────────────────────────┐│
│  │              WaveMilestone Contract                   ││
│  │  ┌──────────────┐  ┌──────────────┐  ┌────────────┐ ││
│  │  │   Instance    │  │  Persistent  │  │   Events   │ ││
│  │  │   Storage     │  │   Storage    │  │   Emitter  │ ││
│  │  │  (Pool Meta)  │  │ (IssueClaim) │  │            │ ││
│  │  └──────────────┘  └──────────────┘  └────────────┘ ││
│  └───────────────────────┬──────────────────────────────┘│
│                          │                               │
│  ┌───────────────────────▼──────────────────────────────┐│
│  │              WaveGuard Contract                       ││
│  │           (Access Registry / Auth)                    ││
│  └───────────────────────┬──────────────────────────────┘│
│                          │                               │
│  ┌───────────────────────▼──────────────────────────────┐│
│  │           Stellar Asset Contract (SAC)                ││
│  │              (Token Transfers)                        ││
│  └──────────────────────────────────────────────────────┘│
└──────────────────────────────────────────────────────────┘
```

## Smart Contract Architecture

### Contract Composition

The `WaveMilestoneContract` is a single Soroban contract with three public lifecycle methods and three view methods:

| Method | Category | Description |
|--------|----------|-------------|
| `create_milestone_pool` | Lifecycle | Initialize escrow vault, lock funds, set expiry |
| `release_issue_bounty` | Lifecycle | Release micro-payout per completed issue |
| `clawback_expired_funds` | Lifecycle | Return unclaimed funds after expiry |
| `milestone_balance` | View | Query remaining pool balance |
| `is_claimed` | View | Check if an issue was already paid |
| `milestone_info` | View | Get full pool metadata |

### Cross-Contract Dependencies

1. **WaveGuard** (`guard_contract`)
   - Interface method: `is_maintainer(address) -> bool`
   - Called on every `create_milestone_pool` and `release_issue_bounty` invocation.

2. **Stellar Asset Contract (SAC)** (`asset`)
   - Interface method: `transfer(from, to, amount)`
   - Called during pool creation (funding) and bounty release (payout).

## Storage Architecture

### Instance Storage (Persistent, Contract Lifetime)

```rust
DataKey::Pool -> MilestonePool {
    guard_contract: Address,
    asset: Address,
    total_funds: u128,
    allocated_funds: u128,
    expiry: u64,
    maintainer: Address,
}
```

- **Bumped on every write** (create_pool, release_bounty, clawback).
- Stores aggregate pool state; read-heavy access for view methods.

### Persistent Storage (Indefinite, Security-Critical)

```rust
DataKey::IssueClaim(BytesN<32>, u32) -> IssueClaim {
    issue_id: u32,
    developer: Address,
    payment_amount: u128,
    completed: bool,
    maintainer: Address,
    claimed_at: u64,
}
```

- **Security fix (CM-01)**: Originally stored in Temporary storage, which allowed
  replay attacks after TTL expiry. Migrated to Persistent storage so that
  duplicate-claim guards are durable for the contract's lifetime.
- **Indefinite lifetime**: Entries survive for the contract's lifetime, not subject
  to automatic TTL pruning.
- **Write-once**: Each `(repo_hash, issue_id)` pair is written exactly once (when
  claimed) and never updated.
- **Audit trail**: Each record captures the `maintainer` who authorized the payout
  and the `claimed_at` ledger timestamp for off-chain auditing.

### Storage Tier Comparison

| Criteria | Instance | Persistent |
|----------|----------|------------|
| Lifetime | Contract lifetime | Contract lifetime |
| Read frequency | High (view methods) | Low (only on claim) |
| Update frequency | Medium (per claim) | Never (write-once) |
| Cost | Higher per byte | Lower per byte |
| Data criticality | Pool integrity | Replay protection |
| Security concern | N/A | Must not use Temporary (CM-01) |

## Authentication & Authorization

### Dual Validation Flow

```
Client TX ──► maintainer.require_auth() ──► WaveGuard.is_maintainer()
                    │
                    ├── Signature verified  ──► Pass
                    ├── Signature invalid   ──► Revert
                    │
                    ▼
            WaveGuard check
                    │
                    ├── Registered maintainer ──► Authorized
                    ├── Unregistered         ──► UnauthorizedMaintainer
                    │
                    ▼
            Clawback only: caller == pool.maintainer
                    │
                    ├── Match  ──► Authorized
                    ├── No match ──► UnauthorizedCaller
```

1. **Transaction-level auth**: `Address::require_auth()` ensures the transaction is signed by the claimed maintainer.
2. **Registry-level auth**: WaveGuard cross-contract call verifies the signer is an active, non-revoked maintainer.
3. **Pool-level auth** (clawback only): The clawback caller must match the `pool.maintainer` exactly.

## Security Properties

### Duplicate Claim Prevention

- Storage key: `DataKey::IssueClaim(repo_hash, issue_id)` — composite of repo identity and issue number.
- Once `completed == true`, all subsequent `release_issue_bounty` calls with the same key revert with `BountyAlreadyClaimed`.
- This prevents drain attacks via replay of claim transactions.

### Balance Overflow Protection

- Every `release_issue_bounty` checks `amount <= pool.remaining_balance()` before any transfer.
- If the check fails, the transaction reverts with `InsufficientPoolBalance` — no tokens are moved, pool state is unchanged.
- This prevents accidental or malicious over-allocation from locking remaining funds.

### Maintainer Revocation

- WaveGuard is the single source of truth for maintainer identity.
- If a maintainer is removed from WaveGuard mid-milestone, all subsequent `release_issue_bounty` calls from that address revert with `UnauthorizedMaintainer`.
- Already-claimed bounties are unaffected (finality is preserved).

## Data Flow: Full Lifecycle

```
1. SETUP PHASE
   Maintainer ──► Deploy WaveMilestone + WaveGuard
               ──► Register as maintainer in WaveGuard
               ──► Mint/lock funds

2. POOL CREATION
   Maintainer ──► create_milestone_pool(guard, asset, total_funds, expiry)
               │
               ├── require_auth()
               ├── WaveGuard.is_maintainer() ✓
               ├── Token.transfer(maintainer → contract, total_funds)
               ├── Storage: Pool { total_funds, allocated_funds: 0, ... }
               └── Event: MilestonePoolCreated

3. BOUNTY RELEASE (per issue)
   Maintainer ──► release_issue_bounty(repo_hash, issue_id, developer, amount)
               │
               ├── require_auth()
               ├── WaveGuard.is_maintainer() ✓
               ├── Storage: Key(issue_id).completed == false
               ├── amount <= remaining_balance() ✓
               ├── Token.transfer(contract → developer, amount)
               ├── Storage: Pool.allocated_funds += amount
               ├── Storage: IssueClaim { completed: true }
               └── Event: BountyReleased

4. CLAWBACK (after expiry)
   Maintainer ──► clawback_expired_funds()
               │
               ├── require_auth()
               ├── caller == pool.maintainer ✓
               ├── now >= pool.expiry ✓
               ├── remaining > 0 ✓
               ├── Token.transfer(contract → maintainer, remaining)
               ├── Storage: Pool.total_funds = Pool.allocated_funds
               └── Event: FundsClawedBack
```

## Testing Architecture

### Test Layers

| Layer | Location | Scope |
|-------|----------|-------|
| Unit tests | `src/test.rs` | Individual function correctness, edge cases |
| Integration tests | `tests/*.rs` | Cross-contract interactions, lifecycle scenarios |
| Mock contracts | `tests/common/` | MockToken, MockWaveGuard for deterministic testing |

### Mock Contracts

**MockToken**: Simulates SAC token behavior with in-storage balance tracking. Supports `mint`, `transfer`, `balance` — enough for full lifecycle testing.

**MockWaveGuard**: Simple boolean registry. Supports `add_maintainer`, `remove_maintainer`, `is_maintainer` — enables testing of access control scenarios.

### Test Scenarios

See [README](../README.md#testing) for the full matrix of test scenarios.
