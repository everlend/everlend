use everlend_utils::EverlendError;
use solana_program::instruction::InstructionError;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;
use solana_program::system_instruction;
use solana_program_test::*;
use solana_sdk::signer::Signer;
use solana_sdk::transaction::{Transaction, TransactionError};

use everlend_depositor::state::{AccountType, Depositor};

use crate::utils::*;

#[tokio::test]
async fn success() {
    let mut env = presetup().await;

    let test_liquidity_oracle = TestLiquidityOracle::new();
    test_liquidity_oracle.init(&mut env.context).await.unwrap();

    let general_pool_market = TestGeneralPoolMarket::new();
    general_pool_market
        .init(&mut env.context, &env.registry.keypair.pubkey())
        .await
        .unwrap();

    let income_pool_market = TestIncomePoolMarket::new();
    income_pool_market
        .init(&mut env.context, &general_pool_market)
        .await
        .unwrap();

    let test_depositor = TestDepositor::new();
    test_depositor
        .init(&mut env.context, &env.registry)
        .await
        .unwrap();

    let depositor = test_depositor.get_data(&mut env.context).await;

    assert_eq!(depositor.account_type, AccountType::Depositor);
}

#[tokio::test]
async fn fail_second_time_init() {
    let mut env = presetup().await;

    let test_liquidity_oracle = TestLiquidityOracle::new();
    test_liquidity_oracle.init(&mut env.context).await.unwrap();

    let general_pool_market = TestGeneralPoolMarket::new();
    general_pool_market
        .init(&mut env.context, &env.registry.keypair.pubkey())
        .await
        .unwrap();

    let income_pool_market = TestIncomePoolMarket::new();
    income_pool_market
        .init(&mut env.context, &general_pool_market)
        .await
        .unwrap();

    let test_depositor = TestDepositor::new();
    test_depositor
        .init(&mut env.context, &env.registry)
        .await
        .unwrap();

    let depositor = test_depositor.get_data(&mut env.context).await;

    assert_eq!(depositor.account_type, AccountType::Depositor);

    let tx = Transaction::new_signed_with_payer(
        &[everlend_depositor::instruction::init(
            &everlend_depositor::id(),
            &env.registry.keypair.pubkey(),
            &test_depositor.depositor.pubkey(),
            &env.context.payer.pubkey(),
        )],
        Some(&env.context.payer.pubkey()),
        &[&env.context.payer],
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
    );
}

#[tokio::test]
async fn fail_with_invalid_registry() {
    let mut env = presetup().await;

    let test_liquidity_oracle = TestLiquidityOracle::new();
    test_liquidity_oracle.init(&mut env.context).await.unwrap();

    let general_pool_market = TestGeneralPoolMarket::new();
    general_pool_market
        .init(&mut env.context, &env.registry.keypair.pubkey())
        .await
        .unwrap();

    let income_pool_market = TestIncomePoolMarket::new();
    income_pool_market
        .init(&mut env.context, &general_pool_market)
        .await
        .unwrap();

    let test_depositor = TestDepositor::new();

    let rent = env.context.banks_client.get_rent().await.unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &env.context.payer.pubkey(),
                &test_depositor.depositor.pubkey(),
                rent.minimum_balance(Depositor::LEN),
                Depositor::LEN as u64,
                &everlend_depositor::id(),
            ),
            everlend_depositor::instruction::init(
                &everlend_depositor::id(),
                &Pubkey::new_unique(),
                &test_depositor.depositor.pubkey(),
                &env.context.payer.pubkey(),
            ),
        ],
        Some(&env.context.payer.pubkey()),
        &[&env.context.payer, &test_depositor.depositor],
        env.context.last_blockhash,
    );

    assert_eq!(
        env.context
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(
            1,
            InstructionError::Custom(EverlendError::InvalidAccountOwner as u32)
        )
    );
}

#[tokio::test]
async fn fail_with_invalid_uncreated_depositor_account() {
    let mut env = presetup().await;

    let test_liquidity_oracle = TestLiquidityOracle::new();
    test_liquidity_oracle.init(&mut env.context).await.unwrap();

    let general_pool_market = TestGeneralPoolMarket::new();
    general_pool_market
        .init(&mut env.context, &env.registry.keypair.pubkey())
        .await
        .unwrap();

    let income_pool_market = TestIncomePoolMarket::new();
    income_pool_market
        .init(&mut env.context, &general_pool_market)
        .await
        .unwrap();

    let test_depositor = TestDepositor::new();

    let tx = Transaction::new_signed_with_payer(
        &[everlend_depositor::instruction::init(
            &everlend_depositor::id(),
            &env.registry.keypair.pubkey(),
            &test_depositor.depositor.pubkey(),
            &env.context.payer.pubkey(),
        )],
        Some(&env.context.payer.pubkey()),
        &[&env.context.payer],
        env.context.last_blockhash,
    );

    assert_eq!(
        env.context
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(EverlendError::InvalidAccountOwner as u32)
        )
    );
}
