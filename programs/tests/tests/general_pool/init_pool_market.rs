#![cfg(feature = "test-bpf")]

use crate::utils::general_pool_market::TestGeneralPoolMarket;
use crate::utils::*;
use everlend_general_pool::instruction;
use everlend_general_pool::state::AccountType;
use solana_program::instruction::InstructionError;
use solana_program_test::*;
use solana_sdk::signer::Signer;
use solana_sdk::transaction::{Transaction, TransactionError};

#[tokio::test]
async fn success() {
    let mut context = presetup().await.0;

    let test_pool_market = TestGeneralPoolMarket::new();
    test_pool_market.init(&mut context).await.unwrap();

    let pool_market = test_pool_market.get_data(&mut context).await;

    assert_eq!(pool_market.account_type, AccountType::PoolMarket);
}

#[tokio::test]
async fn fail_second_time_init() {
    let mut context = presetup().await.0;

    let test_pool_market = TestGeneralPoolMarket::new();
    test_pool_market.init(&mut context).await.unwrap();

    let pool_market = test_pool_market.get_data(&mut context).await;

    assert_eq!(pool_market.account_type, AccountType::PoolMarket);

    context.warp_to_slot(3).unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[instruction::init_pool_market(
            &everlend_general_pool::id(),
            &test_pool_market.keypair.pubkey(),
            &test_pool_market.manager.pubkey(),
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
        TransactionError::InstructionError(0, InstructionError::AccountAlreadyInitialized)
    );
}
