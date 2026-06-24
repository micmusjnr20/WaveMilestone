mod common;

use common::*;
use wave_milestone::types::Error;

#[test]
fn test_duplicate_claim_same_repo_same_issue_rejected() {
    let ctx = TestContext::new();
    ctx.fund_pool(DEFAULT_POOL_FUNDS);

    ctx.client().release_issue_bounty(&ctx.maintainer, &ctx.repo_hash, &1u32, &ctx.developer, &DEFAULT_BOUNTY);

    let result =
        ctx.client().try_release_issue_bounty(&ctx.maintainer, &ctx.repo_hash, &1u32, &ctx.developer, &DEFAULT_BOUNTY);

    assert_eq!(result.err().unwrap(), Ok(Error::BountyAlreadyClaimed));
}

#[test]
fn test_duplicate_claim_different_issue_same_repo_allowed() {
    let ctx = TestContext::new();
    ctx.fund_pool(DEFAULT_POOL_FUNDS);

    ctx.client().release_issue_bounty(&ctx.maintainer, &ctx.repo_hash, &1u32, &ctx.developer, &DEFAULT_BOUNTY);

    // Same repo, different issue — should succeed
    let result =
        ctx.client().try_release_issue_bounty(&ctx.maintainer, &ctx.repo_hash, &2u32, &ctx.developer, &DEFAULT_BOUNTY);

    assert!(result.is_ok());
}

#[test]
fn test_duplicate_claim_same_issue_different_repo_allowed() {
    let ctx = TestContext::new();
    ctx.fund_pool(DEFAULT_POOL_FUNDS * 2);

    ctx.client().release_issue_bounty(&ctx.maintainer, &ctx.repo_hash, &1u32, &ctx.developer, &DEFAULT_BOUNTY);

    // Different repo, same issue number — should succeed
    let result = ctx.client().try_release_issue_bounty(
        &ctx.maintainer,
        &ctx.repo_hash_two,
        &1u32,
        &ctx.developer_two,
        &DEFAULT_BOUNTY,
    );

    assert!(result.is_ok());
}
