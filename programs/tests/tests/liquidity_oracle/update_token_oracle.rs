use crate::utils::*;
use everlend_liquidity_oracle::state::{Distribution, DistributionArray};
use solana_program::clock::Slot;
use solana_program_test::*;
use solana_sdk::{signature::Keypair, signer::Signer};

// const TOKEN_MINT: Pubkey = Pubkey::new_unique();
const WARP_SLOT: Slot = 3;

#[tokio::test]
async fn success() {
    let mut context = program_test().start_with_context().await;
    let token_mint = Keypair::new();
    let payer_pubkey = context.payer.pubkey();

    create_mint(&mut context, &token_mint, &payer_pubkey)
        .await
        .unwrap();

    let test_liquidity_oracle = TestLiquidityOracle::new();
    test_liquidity_oracle.init(&mut context).await.unwrap();

    context.warp_to_slot(WARP_SLOT).unwrap();

    let mut distribution = DistributionArray::default();
    distribution[0] = 100u64;

    let test_token_oracle = TestTokenOracle::new(token_mint.pubkey(), distribution);
    let authority = context.payer.pubkey();

    test_token_oracle
        .init(&mut context, &test_liquidity_oracle, authority)
        .await
        .unwrap();

    context.warp_to_slot(WARP_SLOT + 2).unwrap();

    distribution[0] = 90u64;
    distribution[1] = 10u64;

    test_token_oracle
        .update(
            &mut context,
            &test_liquidity_oracle,
            authority,
            distribution,
        )
        .await
        .unwrap();

    context.warp_to_slot(WARP_SLOT + 4).unwrap();

    let result_distribution = test_token_oracle
        .get_data(
            &mut context,
            &everlend_liquidity_oracle::id(),
            &test_liquidity_oracle,
        )
        .await;

    assert_eq!(
        result_distribution.liquidity_distribution,
        Distribution {
            values: distribution,
            updated_at: WARP_SLOT + 2
        }
    );
}
