# WaveMilestone Security

## Threat Model

WaveMilestone is a smart contract that holds real assets in escrow. The primary security goals are:

1. **Fund Safety**: No one except authorized maintainers can move funds out of the contract.
2. **Fair Distribution**: Each issue can be claimed exactly once — no double-spend.
3. **Timely Clawback**: Unclaimed funds are returnable to the maintainer after milestone expiry.
4. **Access Control**: Only verified maintainers can trigger payouts.

## Vulnerability Analysis

### 1. Duplicate Claim / Drain Attack

**Risk**: An attacker replays a `release_issue_bounty` transaction to drain the pool.

**Mitigation**:
- Composite storage key `(repo_hash, issue_id)` ensures uniqueness across repositories.
- Once `completed` is set to `true`, any subsequent `release_issue_bounty` with the same key reverts with `BountyAlreadyClaimed`.
- The check occurs *before* any token transfer — even a race condition cannot result in double payment.

### 2. Over-Allocation / Pool Insolvency

**Risk**: A maintainer (or attacker controlling a maintainer key) attempts to allocate more than the pool balance.

**Mitigation**:
- `amount > pool.remaining_balance()` check guards every `release_issue_bounty` call.
- On failure, the entire transaction reverts — no partial state change, no locked funds.
- `remaining_balance()` is computed as `total_funds - allocated_funds` (checked arithmetic via Soroban's overflow protection).

### 3. Unauthorized Maintainer

**Risk**: An attacker without maintainer privileges calls lifecycle methods.

**Mitigation**:
- **Dual validation**: `Address::require_auth()` (cryptographic signature verification) + WaveGuard `is_maintainer()` (registry lookup).
- Short of compromising the maintainer's Stellar private key **and** manipulating the WaveGuard registry, this attack is infeasible.
- WaveGuard can revoke a maintainer at any time; revoked maintainers immediately lose access.

### 4. Premature Clawback

**Risk**: A maintainer drains the pool before issues are settled.

**Mitigation**:
- `clawback_expired_funds` checks `now >= pool.expiry` before allowing any transfer.
- Only the `pool.maintainer` (recorded at pool creation) can call clawback — even other registered maintainers cannot.
- The expiry is set at pool creation and is immutable for the pool's lifetime.

### 5. Transfer Failure Path Coverage

**Risk**: A token transfer reverts mid-execution (e.g., the SAC contract panics or the contract's token balance is unexpectedly zero), leaving the contract in an inconsistent state — storage marked as paid but no tokens delivered, or vice versa.

**Mitigation**:
- The contract follows strict **check-effects-interaction** ordering in every payout path:
  1. All inputs and auth validated (checks).
  2. `pool.allocated_funds` incremented and `IssueClaim.completed` set to `true` (effects).
  3. `token.transfer(...)` called last (interaction).
- If `token.transfer(...)` panics, Soroban's host rolls back the entire transaction — storage writes are also reverted, so no funds are marked paid without being delivered.
- The `TransferFailed` error variant (`Error::TransferFailed = 8`) is reserved for callers and wrappers that need to surface a transfer-specific error; the Soroban host itself guarantees atomicity at the host level.
- `clawback_expired_funds` follows the same ordering: state is zeroed before the transfer, and a host-level panic reverts both atomically.

**Audit verification**:
- Confirm `token.transfer(...)` is always the **last** statement in `release_issue_bounty` and `clawback_expired_funds`.
- Confirm no storage write occurs after a `token.transfer(...)` call.
- Confirm `Error::TransferFailed` is not silently swallowed anywhere in calling code.

### 6. Expiry-Based Clawback Security

**Risk**: Incorrect expiry handling allows either (a) a maintainer to reclaim funds prematurely while issues are still open, or (b) funds to be permanently locked if the expiry check contains an off-by-one or is bypassable.

**Mitigation**:
- `clawback_expired_funds` enforces a strict `now >= pool.expiry` check using `env.ledger().timestamp()`, which is the Soroban-provided ledger close time — it cannot be manipulated by the caller.
- Only the address recorded as `pool.maintainer` at pool creation can trigger clawback; other registered maintainers are explicitly rejected (`Error::UnauthorizedCaller`).
- The expiry timestamp is validated at pool creation (`expiry > now`) so a zero or past expiry cannot be stored.
- Once clawback succeeds, `pool.total_funds` is set to `pool.allocated_funds`, effectively closing the pool with a remaining balance of zero — future clawback calls return `Error::NoFundsToClawback` rather than re-entering the transfer path.
- Contributors should be aware that **any unclaimed issue bounties become unclaimable once clawback has executed**. Maintainers must ensure all legitimate claims are settled before the expiry deadline, or use a sufficiently long expiry window (recommend at least 30 days beyond the milestone end date).

**Audit verification**:
- Confirm `now >= pool.expiry` (not `>`) to include the exact expiry ledger second.
- Confirm only `pool.maintainer` — not any WaveGuard-registered maintainer — can call clawback.
- Confirm `pool.expiry > now` is enforced at creation time.
- Confirm post-clawback pool state prevents double-clawback.

### 7. Reentrancy

**Risk**: A malicious token contract calls back into WaveMilestone during `transfer()`.

**Mitigation**:
- The contract follows the **check-effects-interaction** pattern:
  1. Validate inputs and auth (✓)
  2. Update storage state (✓)
  3. Emit events (✓)
  4. Transfer tokens (last)
- Token transfers are the final operation. Any reentrant call would see already-updated state and cannot claim the same issue twice.
- Soroban's environment provides additional reentrancy protection at the host level.

### 8. Front-Running

**Risk**: An attacker observes a pending `release_issue_bounty` and front-runs it with a higher gas payment to claim the bounty for themselves.

**Mitigation**:
- The `developer` address is specified by the maintainer in the transaction parameters.
- Even if an attacker front-runs the transaction, they would be sending funds to the same `developer` address (not to themselves).
- The attacker gains nothing by front-running.

## Trust Assumptions

Understanding what the WaveMilestone contract trusts is critical for a correct security assessment.

### WaveGuard Contract (`guard_contract`)

The WaveGuard registry is the **sole on-chain authority** for maintainer identity. Every privileged write — pool creation and bounty release — defers to `is_maintainer()` on the contract address stored in `pool.guard_contract`.

**Implications**:
- The `guard_contract` address is fixed at `create_milestone_pool` time and **cannot be rotated** afterward. If the WaveGuard instance is compromised, upgraded maliciously, or its `is_maintainer` logic altered, an attacker can gain maintainer status and drain the pool.
- Deployers must ensure WaveGuard is not upgradeable by any party that is not fully trusted, or that any upgrade mechanism requires multi-party approval.
- WaveGuard is **not** consulted during `clawback_expired_funds`. That function performs a direct address equality check (`maintainer == pool.maintainer`) to deliberately isolate the clawback path from a potential WaveGuard compromise.

### Maintainer Address (`maintainer` / `pool.maintainer`)

The `release_issue_bounty` function accepts an arbitrary `developer` address supplied by the maintainer caller. There is **no on-chain restriction** preventing a maintainer from directing a bounty to an address they control.

**Implications**:
- A malicious or compromised maintainer key can redirect any bounty to any address.
- This is an accepted design trade-off: the protocol is permissioned and maintainers are vetted by WaveGuard. Off-chain governance (key management, multi-sig, WaveGuard revocation) is the intended mitigation.
- The expected behavior is documented and tested in `tests/claim_manipulation.rs` (`test_maintainer_can_redirect_developer_address`).

**Mitigations available to deployers**:
- Use a multi-sig wallet for the maintainer key on mainnet.
- Enforce off-chain review/approval before calling `release_issue_bounty`.
- Monitor emitted `BountyReleased` events for unexpected recipient addresses.

### Token Contract (`pool.asset`)

The contract calls an external SAC-style token during fund intake (`create_milestone_pool`), payout (`release_issue_bounty`), and clawback (`clawback_expired_funds`). The token at `pool.asset` is fully trusted to:
- Execute transfers atomically.
- Report accurate balances.
- Not re-enter this contract during transfer calls.
- Not silently absorb or lose funds.

**Implication**: Deployment must use only verified [Stellar Asset Contracts (SAC)](https://developers.stellar.org/docs/tokens/stellar-asset-contract). A malicious or buggy token contract can bypass all other mitigations in this contract.

## Temporary Storage Leakage

### Overview

Stellar's Soroban SDK provides three storage scopes:

| Scope          | Lifetime                          | Use in this contract                 |
|----------------|-----------------------------------|--------------------------------------|
| `instance()`   | Contract instance lifetime        | `MilestonePool` (pool state)         |
| `persistent()` | Indefinite (must be maintained)   | `IssueClaim` (claim records)         |
| `temporary()`  | Expires after configurable TTL    | **Not used for auth-critical state** |

### CM-01: Temporary Storage Claim Expiry (Fixed)

A prior version of the contract stored `IssueClaim` records in **Temporary** storage. Temporary storage entries are pruned by the Stellar network after their TTL lapses. Once pruned, `env.storage().temporary().get(key)` returns `None`.

**Attack vector**: A maintainer releases a bounty for issue `(repo_hash, 42)`. The claim record enters Temporary storage. After the TTL expires (and the entry is pruned), the same maintainer calls `release_issue_bounty` again for the same pair. The guard sees `None`, treats the issue as unclaimed, and releases a second payout — effectively a double-spend.

**Fix**: `IssueClaim` records are now stored in `env.storage().persistent()`. Persistent entries survive for the lifetime of the contract and are not subject to automatic TTL pruning.

**Verification**: See `release_issue_bounty` and `is_claimed` in `contracts/wave_milestone/src/lib.rs`. The `DataKey::IssueClaim` enum variant in `types.rs` includes an explicit `SECURITY` comment requiring `persistent()` access for this key.

### TMP-01: General Rule for Future Development

Any future use of Temporary storage for **authorization state** — one-time nonces, session tokens, permission flags, cooldown markers — is subject to the same expiry-based re-use attack if the TTL is not explicitly managed and checked in application logic.

**Rule**: Authorization-critical state MUST use `instance()` or `persistent()` storage. Temporary storage is appropriate only for **non-security-critical** ephemeral data (e.g., read caches or scratch space) where re-computation after expiry carries no security implication.

### TMP-02: Off-Chain Indexer Compatibility Note

Off-chain indexers or tooling that called `is_claimed()` before the CM-01 fix may see different results on live networks after the fix is deployed:
- Claims recorded in Temporary storage (before the fix) will not be visible via the `is_claimed()` view (which now reads Persistent storage).
- Indexers should treat the contract deployment block as the canonical starting point for Persistent-storage claim records.

## Audit Checklist

Items to verify during security review:

- [ ] `require_auth()` is called on every public method before state mutation.
- [ ] WaveGuard `is_maintainer()` check cannot be bypassed.
- [ ] Composite `(repo_hash, issue_id)` key prevents cross-repo collisions.
- [ ] `BountyAlreadyClaimed` error is emitted before any token transfer.
- [ ] `InsufficientPoolBalance` error reverts entirely — no partial state changes.
- [ ] `clawback_expired_funds` enforces `now >= pool.expiry`.
- [ ] Only `pool.maintainer` can clawback (not any registered maintainer).
- [ ] All arithmetic is checked (Soroban overflow protection enabled in release profile).
- [ ] Events are emitted for all state-changing operations.
- [ ] Zero-amount operations are rejected.
- [ ] Past expiry timestamps are rejected at pool creation.

## Responsible Disclosure

If you discover a security vulnerability in WaveMilestone:

1. **Do not** open a public GitHub issue.
2. **Do not** post details in public forums or discussions.
3. Email the maintainers directly or contact via the [security advisory process](https://github.com/anomalyco/wave-milestone/security/advisories).
4. Provide full details of the vulnerability, including reproduction steps and potential impact.
5. Allow a reasonable period (90 days) for a fix before any public disclosure.

We take all security reports seriously and will acknowledge receipt within 48 hours.

## Bug Bounty

A bug bounty program may be established for high-severity vulnerabilities affecting mainnet deployments. Check the repository discussions for current programs.

## Security-Related Configuration

### Release Profile (Cargo.toml)

```toml
[profile.release]
opt-level = "z"           # Optimize for size
overflow-checks = true    # Runtime integer overflow protection
debug = 0                 # Strip debug symbols
strip = "symbols"         # Strip all symbols
lto = true                # Link-time optimization
codegen-units = 1         # Maximize optimization surface
panic = "abort"           # No unwinding in production
```

### Recommended Maintainer Setup

- Use a **hardware wallet** or **multisig** for the maintainer key on mainnet.
- Rotate maintainer keys periodically via WaveGuard.
- Monitor the contract's `milestone_balance` and event logs regularly.
- Set `expiry` with a generous buffer (e.g., +30 days past the expected milestone end).
