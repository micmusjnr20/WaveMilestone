mod common;

use common::*;

/// End-to-end: full milestone lifecycle from creation through
/// multiple bounty releases to clawback of remaining funds.
#[test]
fn test_full_lifecycle_happy_path() {
    let ctx = TestContext::new();
    let pool_size = 100_000_000_000u128;
    let issues: Vec<(u32, u128)> = vec![
        (1, 10_000_000_000),
        (2, 20_000_000_000),
        (3, 15_000_000_000),
        (4, 25_000_000_000),
        (5, 5_000_000_000),
    ];
    let total_bounties: u128 = issues.iter().map(|(_, a)| a).sum();
    let expected_remaining = pool_size - total_bounties;

    // === Phase 1: Create Pool ===
    ctx.fund_pool(pool_size);

    let info = ctx.client().milestone_info();
    assert!(info.is_some());
    let pool = info.unwrap();
    assert_eq!(pool.total_funds, pool_size);
    assert_eq!(pool.allocated_funds, 0);
    assert_eq!(ctx.client().milestone_balance(), pool_size);

    // === Phase 2: Release Bounties ===
    for (issue_id, amount) in &issues {
        ctx.client().release_issue_bounty(
            &ctx.maintainer,
            &ctx.repo_hash,
            issue_id,
            &ctx.developer,
            amount,
        );
        assert!(ctx.client().is_claimed(&ctx.repo_hash, issue_id));
    }

    assert_eq!(ctx.client().milestone_balance(), expected_remaining);
    assert_eq!(
        ctx.token_client().balance(&ctx.developer),
        total_bounties
    );

    // === Phase 3: Duplicate claims rejected ===
    for (issue_id, _) in &issues {
        let result = ctx.client().try_release_issue_bounty(
            &ctx.maintainer,
            &ctx.repo_hash,
            issue_id,
            &ctx.developer,
            &1_000_000_000,
        );
        assert!(result.is_err());
    }

    // === Phase 4: Clawback After Expiry ===
    let maintainer_before = ctx.token_client().balance(&ctx.maintainer);
    ctx.advance_to_expiry();
    ctx.client().clawback_expired_funds(&ctx.maintainer);
    let maintainer_after = ctx.token_client().balance(&ctx.maintainer);

    assert_eq!(maintainer_after - maintainer_before, expected_remaining);
    assert_eq!(ctx.client().milestone_balance(), 0);
}

/// Edge case: empty milestone (all funds clawed back, no claims).
#[test]
fn test_full_lifecycle_no_claims() {
    let ctx = TestContext::new();
    let pool_size = 50_000_000_000u128;

    ctx.fund_pool(pool_size);
    assert_eq!(ctx.client().milestone_balance(), pool_size);

    ctx.advance_to_expiry();

    let maintainer_before = ctx.token_client().balance(&ctx.maintainer);
    ctx.client().clawback_expired_funds(&ctx.maintainer);
    let maintainer_after = ctx.token_client().balance(&ctx.maintainer);

    assert_eq!(maintainer_after - maintainer_before, pool_size);
    assert_eq!(ctx.client().milestone_balance(), 0);
}

/// Edge case: pool fully claimed, clawback returns nothing.
#[test]
fn test_full_lifecycle_all_claimed() {
    let ctx = TestContext::new();
    let pool_size = 50_000_000_000u128;

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

    ctx.advance_to_expiry();

    let result = ctx
        .client()
        .try_clawback_expired_funds(&ctx.maintainer);
    assert!(result.is_err());
}
