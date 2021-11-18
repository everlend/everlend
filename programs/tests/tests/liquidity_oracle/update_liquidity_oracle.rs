use crate::utils::*;
use solana_program::{instruction::InstructionError, pubkey::Pubkey};
use solana_program_test::*;
use solana_sdk::{signer::Signer, transaction::TransactionError};

async fn setup() -> (ProgramTestContext, TestLiquidityOracle) {
    let mut context = program_test().start_with_context().await;

    let test_liquidity_oracle = TestLiquidityOracle::new();
    test_liquidity_oracle.init(&mut context).await.unwrap();

    (context, test_liquidity_oracle)
}

#[tokio::test]
async fn success() {
    let (mut context, test_liquidity_oracle) = setup().await;
    context.warp_to_slot(3).unwrap();

    let p_k = Pubkey::new_unique();
    test_liquidity_oracle
        .update(&mut context, &p_k)
        .await
        .unwrap();

    let liquidity_oracle = test_liquidity_oracle.get_data(&mut context).await;
    assert_eq!(liquidity_oracle.authority, p_k);
}

#[tokio::test]
async fn fail_wrong_liquidity_oracle_authority() {
    let (mut context, test_liquidity_oracle) = setup().await;
    context.warp_to_slot(3).unwrap();

    let pb_k = Pubkey::new_unique();
    test_liquidity_oracle
        .update(&mut context, &pb_k)
        .await
        .unwrap();

    context.warp_to_slot(5).unwrap();

    let pb_k = context.payer.pubkey();
    assert_eq!(
        test_liquidity_oracle
            .update(&mut context, &pb_k)
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(0, InstructionError::InvalidArgument)
    );
}
