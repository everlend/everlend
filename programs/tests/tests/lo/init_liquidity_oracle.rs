use crate::utils::*;
use everlend_lo::{error::LiquidityOracleError, id, instruction};
use solana_program::instruction::InstructionError;
use solana_program_test::*;
use solana_sdk::{signer::Signer, transaction::Transaction, transaction::TransactionError};

#[tokio::test]
async fn success() {
    let mut context = program_test().start_with_context().await;
    let test_liquidity_oracle = TestLiquidityOracle::new();
    test_liquidity_oracle.init(&mut context).await.unwrap();

    let liquidity_oracle = test_liquidity_oracle.get_data(&mut context).await;
    assert_eq!(liquidity_oracle.authority, context.payer.pubkey());
}

#[tokio::test]
async fn fail_second_time_init() {
    let mut context = program_test().start_with_context().await;
    let test_liquidity_oracle = TestLiquidityOracle::new();
    test_liquidity_oracle.init(&mut context).await.unwrap();

    context.warp_to_slot(3).unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[instruction::init_liquidity_oracle(
            &id(),
            &test_liquidity_oracle.keypair.pubkey(),
            &context.payer.pubkey(),
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );

    assert_eq!(
        context
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(LiquidityOracleError::AlreadyInitialized as u32)
        )
    );
}
