#![cfg(test)]

extern crate std;

use crate::types::Error;
use crate::{WaveMilestoneContract, WaveMilestoneContractClient};
use soroban_sdk::{
    contract, contractimpl, contracttype,
    testutils::{Address as _, Ledger},
    Address, BytesN, Env,
};

// ─────────────────────────────────────────────────────────────
// Mock Contracts
// ─────────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone)]
enum MockGuardKey {
    Maintainer(Address),
}

#[contract]
struct MockWaveGuard;

#[contractimpl]
impl MockWaveGuard {
    pub fn is_maintainer(env: Env, address: Address) -> bool {
        env.storage()
            .instance()
            .get::<_, bool>(&MockGuardKey::Maintainer(address))
            .unwrap_or(false)
    }

    pub fn add_maintainer(env: Env, address: Address) {
        env.storage()
            .instance()
            .set(&MockGuardKey::Maintainer(address), &true);
    }
}

#[contracttype]
#[derive(Clone)]
enum MockTokenKey {
    Balance(Address),
    Admin,
}

#[contract]
struct MockToken;

#[contractimpl]
impl MockToken {
    pub fn init(env: Env, admin: Address) {
        env.storage().instance().set(&MockTokenKey::Admin, &admin);
    }

    pub fn mint(env: Env, to: Address, amount: u128) {
        let bal = env
            .storage()
            .instance()
            .get::<_, u128>(&MockTokenKey::Balance(to.clone()))
            .unwrap_or(0);
        env.storage()
            .instance()
            .set(&MockTokenKey::Balance(to), &(bal + amount));
    }

    pub fn transfer(env: Env, from: Address, to: Address, amount: u128) {
        from.require_auth();
        let from_bal = env
            .storage()
            .instance()
            .get::<_, u128>(&MockTokenKey::Balance(from.clone()))
            .unwrap_or(0);
        if from_bal < amount {
            panic!("insufficient balance");
        }
        let to_bal = env
            .storage()
            .instance()
            .get::<_, u128>(&MockTokenKey::Balance(to.clone()))
            .unwrap_or(0);
        env.storage()
            .instance()
            .set(&MockTokenKey::Balance(from), &(from_bal - amount));
        env.storage()
            .instance()
            .set(&MockTokenKey::Balance(to), &(to_bal + amount));
    }

    pub fn balance(env: Env, id: Address) -> u128 {
        env.storage()
            .instance()
            .get::<_, u128>(&MockTokenKey::Balance(id))
            .unwrap_or(0)
    }
}

// ─────────────────────────────────────────────────────────────
// Test Helpers
// ─────────────────────────────────────────────────────────────

struct TestEnv {
    env: Env,
    maintainer: Address,
    developer: Address,
    stranger: Address,
    contract_id: Address,
    guard_id: Address,
    token_id: Address,
    repo_hash: BytesN<32>,
    expiry: u64,
}

fn setup() -> TestEnv {
    let env = Env::default();
    env.mock_all_auths();

    let maintainer = Address::generate(&env);
    let developer = Address::generate(&env);
    let stranger = Address::generate(&env);

    let guard_id = env.register(MockWaveGuard, ());
    MockWaveGuardClient::new(&env, &guard_id).add_maintainer(&maintainer);

    let token_id = env.register(MockToken, ());
    MockTokenClient::new(&env, &token_id).init(&maintainer);

    let contract_id = env.register(WaveMilestoneContract, ());

    let repo_hash = BytesN::from_array(&env, &[0u8; 32]);
    let expiry = env.ledger().timestamp() + 2_592_000;

    TestEnv {
        env,
        maintainer,
        developer,
        stranger,
        contract_id,
        guard_id,
        token_id,
        repo_hash,
        expiry,
    }
}

fn fund_pool(t: &TestEnv, amount: u128) {
    MockTokenClient::new(&t.env, &t.token_id).mint(&t.maintainer, &amount);
    WaveMilestoneContractClient::new(&t.env, &t.contract_id).create_milestone_pool(
        &t.maintainer,
        &t.guard_id,
        &t.token_id,
        &amount,
        &t.expiry,
    );
}

// ─────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────

#[test]
fn test_create_milestone_pool_success() {
    let t = setup();
    let pool_size: u128 = 10_000_000_000;

    MockTokenClient::new(&t.env, &t.token_id).mint(&t.maintainer, &pool_size);
    WaveMilestoneContractClient::new(&t.env, &t.contract_id).create_milestone_pool(
        &t.maintainer,
        &t.guard_id,
        &t.token_id,
        &pool_size,
        &t.expiry,
    );

    let balance = WaveMilestoneContractClient::new(&t.env, &t.contract_id).milestone_balance();
    assert_eq!(balance, pool_size);

    let info = WaveMilestoneContractClient::new(&t.env, &t.contract_id).milestone_info();
    assert!(info.is_some());
    let pool = info.unwrap();
    assert_eq!(pool.total_funds, pool_size);
    assert_eq!(pool.allocated_funds, 0);
    assert_eq!(pool.maintainer, t.maintainer);
    assert_eq!(pool.asset, t.token_id);
    assert_eq!(pool.guard_contract, t.guard_id);
}

#[test]
fn test_create_pool_rejects_zero_amount() {
    let t = setup();

    let result = WaveMilestoneContractClient::new(&t.env, &t.contract_id).try_create_milestone_pool(
        &t.maintainer,
        &t.guard_id,
        &t.token_id,
        &0u128,
        &t.expiry,
    );

    assert_eq!(result.err().unwrap(), Ok(Error::InvalidAmount));
}

#[test]
fn test_create_pool_rejects_unauthorized_maintainer() {
    let t = setup();
    let pool_size: u128 = 5_000_000_000;

    let result = WaveMilestoneContractClient::new(&t.env, &t.contract_id).try_create_milestone_pool(
        &t.stranger,
        &t.guard_id,
        &t.token_id,
        &pool_size,
        &t.expiry,
    );

    assert_eq!(result.err().unwrap(), Ok(Error::UnauthorizedMaintainer));
}

#[test]
fn test_release_bounty_success() {
    let t = setup();
    let pool_size: u128 = 10_000_000_000;
    let bounty: u128 = 2_500_000_000;

    fund_pool(&t, pool_size);

    let developer_balance_before =
        MockTokenClient::new(&t.env, &t.token_id).balance(&t.developer);

    WaveMilestoneContractClient::new(&t.env, &t.contract_id).release_issue_bounty(
        &t.maintainer,
        &t.repo_hash,
        &1u32,
        &t.developer,
        &bounty,
    );

    let developer_balance_after =
        MockTokenClient::new(&t.env, &t.token_id).balance(&t.developer);
    assert_eq!(developer_balance_after - developer_balance_before, bounty);

    let remaining = WaveMilestoneContractClient::new(&t.env, &t.contract_id).milestone_balance();
    assert_eq!(remaining, pool_size - bounty);

    assert!(
        WaveMilestoneContractClient::new(&t.env, &t.contract_id).is_claimed(&t.repo_hash, &1u32)
    );
}

#[test]
fn test_duplicate_claim_rejected() {
    let t = setup();
    let pool_size: u128 = 10_000_000_000;
    let bounty: u128 = 2_500_000_000;

    fund_pool(&t, pool_size);

    WaveMilestoneContractClient::new(&t.env, &t.contract_id).release_issue_bounty(
        &t.maintainer,
        &t.repo_hash,
        &1u32,
        &t.developer,
        &bounty,
    );

    let result =
        WaveMilestoneContractClient::new(&t.env, &t.contract_id).try_release_issue_bounty(
            &t.maintainer,
            &t.repo_hash,
            &1u32,
            &t.developer,
            &bounty,
        );

    assert_eq!(result.err().unwrap(), Ok(Error::BountyAlreadyClaimed));
}

#[test]
fn test_insufficient_pool_balance_graceful_revert() {
    let t = setup();
    let pool_size: u128 = 1_000_000_000;
    let bounty: u128 = 2_500_000_000;

    fund_pool(&t, pool_size);

    let result =
        WaveMilestoneContractClient::new(&t.env, &t.contract_id).try_release_issue_bounty(
            &t.maintainer,
            &t.repo_hash,
            &1u32,
            &t.developer,
            &bounty,
        );

    assert_eq!(result.err().unwrap(), Ok(Error::InsufficientPoolBalance));

    // Pool should be intact
    let remaining = WaveMilestoneContractClient::new(&t.env, &t.contract_id).milestone_balance();
    assert_eq!(remaining, pool_size);
}

#[test]
fn test_over_allocation_after_partial_claims() {
    let t = setup();
    let pool_size: u128 = 5_000_000_000;
    let first_bounty: u128 = 3_000_000_000;
    let second_bounty: u128 = 3_000_000_000;

    fund_pool(&t, pool_size);

    // First claim should succeed
    WaveMilestoneContractClient::new(&t.env, &t.contract_id).release_issue_bounty(
        &t.maintainer,
        &t.repo_hash,
        &1u32,
        &t.developer,
        &first_bounty,
    );

    // Second claim exceeds remaining 2B
    let result =
        WaveMilestoneContractClient::new(&t.env, &t.contract_id).try_release_issue_bounty(
            &t.maintainer,
            &t.repo_hash,
            &2u32,
            &t.developer,
            &second_bounty,
        );

    assert_eq!(result.err().unwrap(), Ok(Error::InsufficientPoolBalance));
}

#[test]
fn test_clawback_expired_funds() {
    let t = setup();
    let pool_size: u128 = 10_000_000_000;
    let bounty: u128 = 2_500_000_000;

    fund_pool(&t, pool_size);

    // Claim one issue
    WaveMilestoneContractClient::new(&t.env, &t.contract_id).release_issue_bounty(
        &t.maintainer,
        &t.repo_hash,
        &1u32,
        &t.developer,
        &bounty,
    );

    // Jump past expiry
    t.env.ledger().set_timestamp(t.expiry + 1);

    let maintainer_balance_before =
        MockTokenClient::new(&t.env, &t.token_id).balance(&t.maintainer);

    WaveMilestoneContractClient::new(&t.env, &t.contract_id).clawback_expired_funds(&t.maintainer);

    let maintainer_balance_after =
        MockTokenClient::new(&t.env, &t.token_id).balance(&t.maintainer);
    let expected_clawback = pool_size - bounty;
    assert_eq!(
        maintainer_balance_after - maintainer_balance_before,
        expected_clawback
    );

    // Pool should show no remaining spendable balance
    let remaining = WaveMilestoneContractClient::new(&t.env, &t.contract_id).milestone_balance();
    assert_eq!(remaining, 0);
}

#[test]
fn test_clawback_before_expiry_rejected() {
    let t = setup();
    let pool_size: u128 = 10_000_000_000;

    fund_pool(&t, pool_size);

    let result = WaveMilestoneContractClient::new(&t.env, &t.contract_id)
        .try_clawback_expired_funds(&t.maintainer);

    assert_eq!(result.err().unwrap(), Ok(Error::PoolNotExpired));
}

#[test]
fn test_unauthorized_caller_rejected() {
    let t = setup();
    let pool_size: u128 = 10_000_000_000;

    fund_pool(&t, pool_size);

    let result = WaveMilestoneContractClient::new(&t.env, &t.contract_id).try_clawback_expired_funds(
        &t.stranger,
    );

    assert_eq!(result.err().unwrap(), Ok(Error::UnauthorizedCaller));
}

#[test]
fn test_non_maintainer_cannot_create_pool() {
    let t = setup();
    let pool_size: u128 = 10_000_000_000;

    MockTokenClient::new(&t.env, &t.token_id).mint(&t.stranger, &pool_size);

    let result = WaveMilestoneContractClient::new(&t.env, &t.contract_id).try_create_milestone_pool(
        &t.stranger,
        &t.guard_id,
        &t.token_id,
        &pool_size,
        &t.expiry,
    );

    assert_eq!(result.err().unwrap(), Ok(Error::UnauthorizedMaintainer));
}

#[test]
fn test_multiple_issues_different_repos_independent() {
    let t = setup();
    let pool_size: u128 = 10_000_000_000;
    let repo_b = BytesN::from_array(&t.env, &[1u8; 32]);

    fund_pool(&t, pool_size);

    // Claim issue 1 in repo_a
    WaveMilestoneContractClient::new(&t.env, &t.contract_id).release_issue_bounty(
        &t.maintainer,
        &t.repo_hash,
        &1u32,
        &t.developer,
        &1_000_000_000,
    );

    // Claim same issue number in different repo should work
    WaveMilestoneContractClient::new(&t.env, &t.contract_id).release_issue_bounty(
        &t.maintainer,
        &repo_b,
        &1u32,
        &t.developer,
        &2_000_000_000,
    );

    // Each should be independently tracked
    assert!(
        WaveMilestoneContractClient::new(&t.env, &t.contract_id).is_claimed(&t.repo_hash, &1u32)
    );
    assert!(
        WaveMilestoneContractClient::new(&t.env, &t.contract_id).is_claimed(&repo_b, &1u32)
    );
}

#[test]
fn test_pool_not_found_returns_none() {
    let t = setup();

    let info = WaveMilestoneContractClient::new(&t.env, &t.contract_id).milestone_info();
    assert!(info.is_none());

    let balance = WaveMilestoneContractClient::new(&t.env, &t.contract_id).milestone_balance();
    assert_eq!(balance, 0);
}

#[test]
fn test_release_issue_bounty_pool_not_found() {
    let t = setup();

    let result =
        WaveMilestoneContractClient::new(&t.env, &t.contract_id).try_release_issue_bounty(
            &t.maintainer,
            &t.repo_hash,
            &1u32,
            &t.developer,
            &1_000_000_000,
        );

    assert_eq!(result.err().unwrap(), Ok(Error::PoolNotFound));
}
