use crate::utils::*;
use everlend_liquidity_oracle::state::{Distribution, DistributionArray};
use solana_program::{clock::Slot, instruction::InstructionError};
use solana_program_test::*;
use solana_sdk::{signature::Keypair, signer::Signer, transaction::TransactionError};

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
            updated_at: WARP_SLOT
        }
    );
}

#[tokio::test]
async fn fail_second_time_init() {
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

    assert_eq!(
        test_token_oracle
            .init(&mut context, &test_liquidity_oracle, authority)
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(0, InstructionError::AccountAlreadyInitialized)
    );
}

#[tokio::test]
async fn fail_incorrect_max_distribution() {
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
    distribution[0] = 1000000001u64;

    let test_token_oracle = TestTokenOracle::new(token_mint.pubkey(), distribution);
    let authority = context.payer.pubkey();

    assert_eq!(
        test_token_oracle
            .init(&mut context, &test_liquidity_oracle, authority)
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(0, InstructionError::InvalidArgument)
    );
}
