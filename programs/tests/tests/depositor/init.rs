#![cfg(feature = "test-bpf")]

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
    let (mut context, _, _, registry) = presetup().await;

    let test_liquidity_oracle = TestLiquidityOracle::new();
    test_liquidity_oracle.init(&mut context).await.unwrap();

    let general_pool_market = TestGeneralPoolMarket::new();
    general_pool_market.init(&mut context, &registry.keypair.pubkey()).await.unwrap();

    let income_pool_market = TestIncomePoolMarket::new();
    income_pool_market
        .init(&mut context, &general_pool_market)
        .await
        .unwrap();

    let test_depositor = TestDepositor::new();
    test_depositor.init(&mut context, &registry).await.unwrap();

    let depositor = test_depositor.get_data(&mut context).await;

    assert_eq!(depositor.account_type, AccountType::Depositor);
}

#[tokio::test]
async fn fail_second_time_init() {
    let (mut context, _, _, registry) = presetup().await;

    let test_liquidity_oracle = TestLiquidityOracle::new();
    test_liquidity_oracle.init(&mut context).await.unwrap();

    let general_pool_market = TestGeneralPoolMarket::new();
    general_pool_market.init(&mut context, &registry.keypair.pubkey()).await.unwrap();

    let income_pool_market = TestIncomePoolMarket::new();
    income_pool_market
        .init(&mut context, &general_pool_market)
        .await
        .unwrap();

    let test_depositor = TestDepositor::new();
    test_depositor.init(&mut context, &registry).await.unwrap();

    let depositor = test_depositor.get_data(&mut context).await;

    assert_eq!(depositor.account_type, AccountType::Depositor);

    let tx = Transaction::new_signed_with_payer(
        &[everlend_depositor::instruction::init(
            &everlend_depositor::id(),
            &registry.keypair.pubkey(),
            &test_depositor.depositor.pubkey(),
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

#[tokio::test]
async fn fail_with_invalid_registry() {
    let (mut context, _, _, registry) = presetup().await;

    let test_liquidity_oracle = TestLiquidityOracle::new();
    test_liquidity_oracle.init(&mut context).await.unwrap();

    let general_pool_market = TestGeneralPoolMarket::new();
    general_pool_market.init(&mut context, &registry.keypair.pubkey()).await.unwrap();

    let income_pool_market = TestIncomePoolMarket::new();
    income_pool_market
        .init(&mut context, &general_pool_market)
        .await
        .unwrap();

    let test_depositor = TestDepositor::new();

    let rent = context.banks_client.get_rent().await.unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &test_depositor.depositor.pubkey(),
                rent.minimum_balance(Depositor::LEN),
                Depositor::LEN as u64,
                &everlend_depositor::id(),
            ),
            everlend_depositor::instruction::init(
                &everlend_depositor::id(),
                &Pubkey::new_unique(),
                &test_depositor.depositor.pubkey(),
            ),
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, &test_depositor.depositor],
        context.last_blockhash,
    );

    assert_eq!(
        context
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(1,
            InstructionError::Custom(EverlendError::InvalidAccountOwner as u32))
    );
}

#[tokio::test]
async fn fail_with_invalid_uncreated_depositor_account() {
    let (mut context, _, _, registry) = presetup().await;

    let test_liquidity_oracle = TestLiquidityOracle::new();
    test_liquidity_oracle.init(&mut context).await.unwrap();

    let general_pool_market = TestGeneralPoolMarket::new();
    general_pool_market.init(&mut context, &registry.keypair.pubkey()).await.unwrap();

    let income_pool_market = TestIncomePoolMarket::new();
    income_pool_market
        .init(&mut context, &general_pool_market)
        .await
        .unwrap();

    let test_depositor = TestDepositor::new();

    let tx = Transaction::new_signed_with_payer(
        &[everlend_depositor::instruction::init(
            &everlend_depositor::id(),
            &registry.keypair.pubkey(),
            &test_depositor.depositor.pubkey(),
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
        TransactionError::InstructionError(0, InstructionError::AccountNotRentExempt)
    );
}
