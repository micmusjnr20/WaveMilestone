mod common;

use common::*;
use wave_milestone::types::Error;

#[test]
fn test_over_allocation_reverts_gracefully() {
    let ctx = TestContext::new();
    let pool_size = 1_000_000_000u128;
    let oversized_bounty = 2_500_000_000u128;

    ctx.fund_pool(pool_size);

    let result = ctx.client().try_release_issue_bounty(
        &ctx.maintainer,
        &ctx.repo_hash,
        &1u32,
        &ctx.developer,
        &oversized_bounty,
    );

    assert_eq!(result.err().unwrap(), Ok(Error::InsufficientPoolBalance));

    // Pool must remain fully intact after failed over-allocation
    assert_eq!(ctx.client().milestone_balance(), pool_size);
    assert_eq!(ctx.token_client().balance(&ctx.developer), 0);
}

#[test]
fn test_exact_balance_allocation_succeeds() {
    let ctx = TestContext::new();
    let pool_size = 5_000_000_000u128;

    ctx.fund_pool(pool_size);

    ctx.client().release_issue_bounty(
        &ctx.maintainer,
        &ctx.repo_hash,
        &1u32,
        &ctx.developer,
        &pool_size,
    );

    assert_eq!(ctx.client().milestone_balance(), 0);
    assert_eq!(ctx.token_client().balance(&ctx.developer), pool_size);
}

#[test]
fn test_partial_claim_then_over_allocate_remaining() {
    let ctx = TestContext::new();
    let pool_size = 5_000_000_000u128;
    let first = 3_000_000_000u128;
    let second = 3_000_000_000u128;

    ctx.fund_pool(pool_size);

    ctx.client().release_issue_bounty(
        &ctx.maintainer,
        &ctx.repo_hash,
        &1u32,
        &ctx.developer,
        &first,
    );

    let result = ctx.client().try_release_issue_bounty(
        &ctx.maintainer,
        &ctx.repo_hash,
        &2u32,
        &ctx.developer,
        &second,
    );

    assert_eq!(result.err().unwrap(), Ok(Error::InsufficientPoolBalance));

    // First claim should still be intact
    assert_eq!(ctx.token_client().balance(&ctx.developer), first);
    assert_eq!(ctx.client().milestone_balance(), pool_size - first);
}

#[test]
fn test_zero_amount_does_not_drain_pool() {
    let ctx = TestContext::new();
    ctx.fund_pool(DEFAULT_POOL_FUNDS);

    let result = ctx.client().try_release_issue_bounty(
        &ctx.maintainer,
        &ctx.repo_hash,
        &1u32,
        &ctx.developer,
        &0u128,
    );

    assert_eq!(result.err().unwrap(), Ok(Error::InvalidAmount));
    assert_eq!(ctx.client().milestone_balance(), DEFAULT_POOL_FUNDS);
}
