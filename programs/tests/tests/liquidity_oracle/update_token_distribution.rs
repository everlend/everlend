use crate::utils::*;
use everlend_liquidity_oracle::state::DistributionArray;
use solana_program::{clock::Slot, pubkey::Pubkey};
use solana_program_test::*;
use solana_sdk::signer::Signer;

// const TOKEN_MINT: Pubkey = Pubkey::new_unique();
const WARP_SLOT: Slot = 3;

#[tokio::test]
async fn success() {
    let mut context = program_test().start_with_context().await;
    let token_mint: Pubkey = Pubkey::new_unique();
    let test_liquidity_oracle = TestLiquidityOracle::new();
    test_liquidity_oracle.init(&mut context).await.unwrap();

    context.warp_to_slot(WARP_SLOT).unwrap();

    let mut distribution = DistributionArray::default();
    distribution[0] = 100u64;

    let test_token_distribution = TestTokenDistribution::new(token_mint, distribution);
    let authority = context.payer.pubkey();

    test_token_distribution
        .init(&mut context, &test_liquidity_oracle, authority)
        .await
        .unwrap();

    context.warp_to_slot(WARP_SLOT + 2).unwrap();

    distribution[0] = 90u64;
    distribution[1] = 10u64;

    test_token_distribution
        .update(
            &mut context,
            &test_liquidity_oracle,
            authority,
            distribution,
        )
        .await
        .unwrap();

    context.warp_to_slot(WARP_SLOT + 4).unwrap();

    let result_distribution = test_token_distribution
        .get_data(
            &mut context,
            &everlend_liquidity_oracle::id(),
            &test_liquidity_oracle,
        )
        .await;

    assert_eq!(distribution, result_distribution.distribution);
    assert_eq!(WARP_SLOT + 2, result_distribution.updated_at);
}
