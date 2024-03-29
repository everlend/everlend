use solana_program::instruction::InstructionError;
use solana_program_test::*;
use solana_sdk::signer::Signer;
use solana_sdk::transaction::{Transaction, TransactionError};

use everlend_income_pools::instruction;
use everlend_income_pools::state::AccountType;

use crate::utils::*;

#[tokio::test]
async fn success() {
    let mut env = presetup().await;

    let general_pool_market = TestGeneralPoolMarket::new();
    general_pool_market
        .init(&mut env.context, &env.registry.keypair.pubkey())
        .await
        .unwrap();

    let test_income_pool_market = TestIncomePoolMarket::new();
    test_income_pool_market
        .init(&mut env.context, &general_pool_market)
        .await
        .unwrap();

    let pool_market = test_income_pool_market.get_data(&mut env.context).await;

    assert_eq!(pool_market.account_type, AccountType::IncomePoolMarket);
}

#[tokio::test]
async fn fail_second_time_init() {
    let mut env = presetup().await;

    let general_pool_market = TestGeneralPoolMarket::new();
    general_pool_market
        .init(&mut env.context, &env.registry.keypair.pubkey())
        .await
        .unwrap();

    let test_income_pool_market = TestIncomePoolMarket::new();
    test_income_pool_market
        .init(&mut env.context, &general_pool_market)
        .await
        .unwrap();

    let pool_market = test_income_pool_market.get_data(&mut env.context).await;

    assert_eq!(pool_market.account_type, AccountType::IncomePoolMarket);

    let tx = Transaction::new_signed_with_payer(
        &[instruction::init_pool_market(
            &everlend_income_pools::id(),
            &test_income_pool_market.keypair.pubkey(),
            &test_income_pool_market.manager.pubkey(),
            &general_pool_market.keypair.pubkey(),
        )],
        Some(&env.context.payer.pubkey()),
        &[&env.context.payer, &test_income_pool_market.manager],
        env.context.last_blockhash,
    );

    assert_eq!(
        env.context
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(0, InstructionError::AccountAlreadyInitialized)
    )
}
