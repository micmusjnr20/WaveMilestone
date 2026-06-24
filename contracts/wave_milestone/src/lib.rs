#![no_std]

mod events;
mod test;
pub mod types;

use events::{BountyReleasedEvent, FundsClawedBackEvent, PoolCreatedEvent};
use soroban_sdk::{contract, contractimpl, symbol_short, Address, BytesN, Env};
use types::{DataKey, Error, IssueClaim, MilestonePool, TokenClient, WaveGuardClient};

const ONE_MONTH_IN_SECONDS: u64 = 2_592_000;

// ─────────────────────────────────────────────────────────────
// Contract Entry Point
// ─────────────────────────────────────────────────────────────

#[contract]
pub struct WaveMilestoneContract;

#[contractimpl]
impl WaveMilestoneContract {
    // ── Lifecycle: Pool Creation ─────────────────────────────

    /// Creates a new milestone escrow pool.
    ///
    /// Transfers `total_funds` of `asset` from `maintainer` into the
    /// contract vault, links to a WaveGuard registry for access control,
    /// and sets a milestone `expiry` (ledger timestamp, Unix seconds).
    ///
    /// # Auth
    /// - `maintainer.require_auth()` — the caller must sign.
    /// - WaveGuard `is_maintainer` check passes.
    pub fn create_milestone_pool(
        env: Env,
        maintainer: Address,
        guard_contract: Address,
        asset: Address,
        total_funds: u128,
        expiry: u64,
    ) -> Result<(), Error> {
        // ── Authentication ──
        maintainer.require_auth();

        // ── WaveGuard validation ──
        let guard = WaveGuardClient::new(&env, &guard_contract);
        if !guard.is_maintainer(&maintainer) {
            return Err(Error::UnauthorizedMaintainer);
        }

        // ── Input validation ──
        if total_funds == 0 {
            return Err(Error::InvalidAmount);
        }
        let now = env.ledger().timestamp();
        if expiry <= now {
            return Err(Error::ExpiryInPast);
        }

        // ── Fund transfer ──
        let token = TokenClient::new(&env, &asset);
        token.transfer(&maintainer, &env.current_contract_address(), &total_funds);

        // ── Persist pool ──
        let pool = MilestonePool {
            guard_contract,
            asset: asset.clone(),
            total_funds,
            allocated_funds: 0,
            expiry,
            maintainer: maintainer.clone(),
        };
        env.storage().instance().set(&DataKey::Pool, &pool);

        // ── Emit event ──
        env.events().publish(
            (symbol_short!("create"),),
            PoolCreatedEvent {
                maintainer,
                asset,
                total_funds,
                expiry,
            },
        );

        Ok(())
    }

    // ── Lifecycle: Bounty Release ────────────────────────────

    /// Releases a micro-payout to `developer` for a completed issue.
    ///
    /// Each `(repo_hash, issue_id)` pair can be claimed exactly once.
    /// The contract verifies the maintainer's identity via WaveGuard,
    /// checks the issue has not already been paid, confirms sufficient
    /// pool balance, then transfers the tokens and marks the claim.
    ///
    /// # Auth
    /// - `maintainer.require_auth()` — the caller must sign.
    /// - WaveGuard `is_maintainer` check passes.
    pub fn release_issue_bounty(
        env: Env,
        maintainer: Address,
        repo_hash: BytesN<32>,
        issue_id: u32,
        developer: Address,
        amount: u128,
    ) -> Result<(), Error> {
        // ── Authentication ──
        maintainer.require_auth();

        // ── Load pool ──
        let mut pool = env
            .storage()
            .instance()
            .get::<_, MilestonePool>(&DataKey::Pool)
            .ok_or(Error::PoolNotFound)?;

        // ── WaveGuard validation ──
        let guard = WaveGuardClient::new(&env, &pool.guard_contract);
        if !guard.is_maintainer(&maintainer) {
            return Err(Error::UnauthorizedMaintainer);
        }

        // ── Duplicate-claim guard ──
        let claim_key = DataKey::IssueClaim(repo_hash.clone(), issue_id);
        if let Some(claim) = env
            .storage()
            .temporary()
            .get::<_, IssueClaim>(&claim_key)
        {
            if claim.completed {
                return Err(Error::BountyAlreadyClaimed);
            }
        }

        // ── Balance check ──
        let remaining = pool.remaining_balance();
        if amount > remaining {
            return Err(Error::InsufficientPoolBalance);
        }
        if amount == 0 {
            return Err(Error::InvalidAmount);
        }

        // ── Transfer tokens ──
        let token = TokenClient::new(&env, &pool.asset);
        token.transfer(
            &env.current_contract_address(),
            &developer,
            &amount,
        );

        // ── Update pool state ──
        pool.allocated_funds += amount;
        env.storage()
            .instance()
            .set(&DataKey::Pool, &pool);

        // ── Record claim ──
        let claim = IssueClaim {
            issue_id,
            developer: developer.clone(),
            payment_amount: amount,
            completed: true,
        };
        env.storage()
            .temporary()
            .set(&claim_key, &claim);

        // ── Emit event ──
        env.events().publish(
            (symbol_short!("release"),),
            BountyReleasedEvent {
                repo_hash,
                issue_id,
                developer,
                amount,
            },
        );

        Ok(())
    }

    // ── Lifecycle: Clawback ──────────────────────────────────

    /// Returns unclaimed funds to the maintainer after milestone expiry.
    ///
    /// Only callable by the original `pool.maintainer` and only after
    /// `pool.expiry` has passed. Transfers the full remaining balance
    /// back to the maintainer and zeroes out the available pool.
    pub fn clawback_expired_funds(env: Env, maintainer: Address) -> Result<(), Error> {
        maintainer.require_auth();

        let mut pool = env
            .storage()
            .instance()
            .get::<_, MilestonePool>(&DataKey::Pool)
            .ok_or(Error::PoolNotFound)?;

        if maintainer != pool.maintainer {
            return Err(Error::UnauthorizedCaller);
        }

        let now = env.ledger().timestamp();
        if now < pool.expiry {
            return Err(Error::PoolNotExpired);
        }

        let remaining = pool.remaining_balance();
        if remaining == 0 {
            return Err(Error::NoFundsToClawback);
        }

        let token = TokenClient::new(&env, &pool.asset);
        token.transfer(
            &env.current_contract_address(),
            &maintainer,
            &remaining,
        );

        pool.total_funds = pool.allocated_funds;
        env.storage()
            .instance()
            .set(&DataKey::Pool, &pool);

        env.events().publish(
            (symbol_short!("clawback"),),
            FundsClawedBackEvent {
                maintainer,
                amount: remaining,
            },
        );

        Ok(())
    }

    // ── View / Query Methods ─────────────────────────────────

    /// Returns the remaining spendable balance in the milestone pool.
    pub fn milestone_balance(env: Env) -> u128 {
        env.storage()
            .instance()
            .get::<_, MilestonePool>(&DataKey::Pool)
            .map(|p| p.remaining_balance())
            .unwrap_or(0)
    }

    /// Returns `true` if a specific issue has already been claimed.
    pub fn is_claimed(env: Env, repo_hash: BytesN<32>, issue_id: u32) -> bool {
        let claim_key = DataKey::IssueClaim(repo_hash, issue_id);
        env.storage()
            .temporary()
            .get::<_, IssueClaim>(&claim_key)
            .map(|c| c.completed)
            .unwrap_or(false)
    }

    /// Returns the full milestone metadata, or `None` if uninitialized.
    pub fn milestone_info(env: Env) -> Option<MilestonePool> {
        env.storage()
            .instance()
            .get::<_, MilestonePool>(&DataKey::Pool)
    }
}
