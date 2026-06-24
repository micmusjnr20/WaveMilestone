use soroban_sdk::{contractclient, contracterror, contracttype, Address, BytesN, Env};

// ─────────────────────────────────────────────────────────────
// Data Structures
// ─────────────────────────────────────────────────────────────

/// Core escrow pool representing a funded milestone.
/// Stored in Instance storage for the contract's lifetime.
#[contracttype]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MilestonePool {
    pub guard_contract: Address,
    pub asset: Address,
    pub total_funds: u128,
    pub allocated_funds: u128,
    pub expiry: u64,
    pub maintainer: Address,
}

impl MilestonePool {
#89-Add-CI-check-for-Soroban-contract-build-FIX
    /// Returns the unallocated amount remaining in the pool.
    #[must_use]
    pub fn remaining_balance(&self) -> u128 {
        self.total_funds - self.allocated_funds

    pub fn remaining_balance(&self) -> u128 {
        self.total_funds.saturating_sub(self.allocated_funds)
main
    }
}

/// Individual issue bounty claim record.
/// Stored in Temporary storage to save gas; single-use lifecycle.
#[contracttype]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssueClaim {
    pub issue_id: u32,
    pub developer: Address,
    pub payment_amount: u128,
    pub completed: bool,
}

// ─────────────────────────────────────────────────────────────
// Storage Keys
// ─────────────────────────────────────────────────────────────

#[contracttype]
pub enum DataKey {
    /// Singleton — the active milestone pool
    Pool,
    /// Per-issue claim under a specific repository
    /// Key: (repo_hash: BytesN<32>, issue_id: u32)
    IssueClaim(BytesN<32>, u32),
}

// ─────────────────────────────────────────────────────────────
// Error Enum
// ─────────────────────────────────────────────────────────────

#[contracterror]
#89-Add-CI-check-for-Soroban-contract-build-FIX
#[derive(Debug, Copy, Clone, PartialEq, Eq)]

#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
main
#[repr(u32)]
pub enum Error {
    PoolNotFound = 1,
    PoolNotExpired = 2,
    BountyAlreadyClaimed = 3,
    InsufficientPoolBalance = 4,
    UnauthorizedMaintainer = 5,
    UnauthorizedCaller = 6,
    NoFundsToClawback = 7,
    TransferFailed = 8,
    InvalidAmount = 9,
    ExpiryInPast = 10,
}

// ─────────────────────────────────────────────────────────────
// Cross-Contract Interfaces
// ─────────────────────────────────────────────────────────────

/// WaveGuard identity registry interface.
/// Verifies that an address is an authorized maintainer.
#[contractclient(name = "WaveGuardClient")]
pub trait WaveGuardInterface {
    fn is_maintainer(env: Env, address: Address) -> bool;
}

/// Standard Stellar Asset Contract (SAC) token interface.
#[contractclient(name = "TokenClient")]
pub trait TokenInterface {
    fn transfer(env: Env, from: Address, to: Address, amount: u128);
    fn balance(env: Env, id: Address) -> u128;
    fn xfer(env: Env, from: Address, to: Address, amount: i128);
}
