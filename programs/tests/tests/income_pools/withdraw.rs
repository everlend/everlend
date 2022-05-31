#![cfg(feature = "test-bpf")]

use solana_program::instruction::InstructionError;
use solana_program::pubkey::Pubkey;
use solana_program_test::*;
use solana_sdk::signer::Signer;
use solana_sdk::transaction::{Transaction, TransactionError};

use everlend_income_pools::instruction;
use everlend_utils::EverlendError;

use crate::utils::*;

const TOKEN_AMOUNT: u64 = 123 * EXP;
const SAFETY_FUND_AMOUNT: u64 = 246 * 10000000;

async fn setup() -> (
    ProgramTestContext,
    TestIncomePoolMarket,
    TestIncomePool,
    TestGeneralPool,
) {
    let (mut context, _, _, registry) = presetup().await;

    let general_pool_market = TestGeneralPoolMarket::new();
    general_pool_market.init(&mut context, &registry.keypair.pubkey()).await.unwrap();

    let test_income_pool_market = TestIncomePoolMarket::new();
    test_income_pool_market
        .init(&mut context, &general_pool_market)
        .await
        .unwrap();

    let test_general_pool = TestGeneralPool::new(&general_pool_market, None);
    test_general_pool
        .create(&mut context, &general_pool_market)
        .await
        .unwrap();

    let test_income_pool = TestIncomePool::new(&test_income_pool_market, None);
    test_income_pool
        .create(&mut context, &test_income_pool_market)
        .await
        .unwrap();

    test_income_pool
        .create_safety_fund_token_account(
            &mut context,
            &test_income_pool_market,
            &test_general_pool,
        )
        .await
        .unwrap();

    mint_tokens(
        &mut context,
        &test_income_pool.token_mint_pubkey,
        &test_income_pool.token_account.pubkey(),
        TOKEN_AMOUNT,
    )
    .await
    .unwrap();

    (
        context,
        test_income_pool_market,
        test_income_pool,
        test_general_pool,
    )
}

#[tokio::test]
async fn success() {
    let (mut context, test_income_pool_market, test_income_pool, test_general_pool) = setup().await;

    assert_eq!(
        get_token_balance(&mut context, &test_income_pool.token_account.pubkey()).await,
        TOKEN_AMOUNT
    );

    test_income_pool
        .withdraw(&mut context, &test_income_pool_market, &test_general_pool)
        .await
        .unwrap();

    assert_eq!(
        get_token_balance(&mut context, &test_general_pool.token_account.pubkey()).await,
        TOKEN_AMOUNT - SAFETY_FUND_AMOUNT
    );

    assert_eq!(
        get_token_balance(
            &mut context,
            &test_income_pool.get_safety_fund_token_account(&test_income_pool_market)
        )
        .await,
        SAFETY_FUND_AMOUNT
    );
}

#[tokio::test]
async fn success_with_zero_balance() {
    let (mut context, test_income_pool_market, test_income_pool, test_general_pool) = setup().await;

    assert_eq!(
        get_token_balance(&mut context, &test_income_pool.token_account.pubkey()).await,
        TOKEN_AMOUNT
    );

    test_income_pool
        .withdraw(&mut context, &test_income_pool_market, &test_general_pool)
        .await
        .unwrap();

    assert_eq!(
        get_token_balance(&mut context, &test_income_pool.token_account.pubkey()).await,
        0
    );

    context.warp_to_slot(3).unwrap();

    //todo fail on withdraw with zero balance?
    test_income_pool
        .withdraw(&mut context, &test_income_pool_market, &test_general_pool)
        .await
        .unwrap();
}

#[tokio::test]
async fn fail_with_invalid_income_pool_market() {
    let (mut context, _, test_income_pool, test_general_pool) = setup().await;

    let tx = Transaction::new_signed_with_payer(
        &[instruction::withdraw(
            &everlend_income_pools::id(),
            &test_general_pool.token_mint_pubkey,
            &Pubkey::new_unique(),
            &test_income_pool.pool_pubkey,
            &test_income_pool.token_account.pubkey(),
            &test_general_pool.pool_pubkey,
            &test_general_pool.token_account.pubkey(),
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
            InstructionError::Custom(EverlendError::InvalidAccountOwner as u32),
        )
    );
}

#[tokio::test]
async fn fail_with_invalid_income_pool() {
    let (mut context, test_income_pool_market, test_income_pool, test_general_pool) = setup().await;

    let tx = Transaction::new_signed_with_payer(
        &[instruction::withdraw(
            &everlend_income_pools::id(),
            &test_general_pool.token_mint_pubkey,
            &test_income_pool_market.keypair.pubkey(),
            &Pubkey::new_unique(),
            &test_income_pool.token_account.pubkey(),
            &test_general_pool.pool_pubkey,
            &test_general_pool.token_account.pubkey(),
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
            InstructionError::Custom(EverlendError::InvalidAccountOwner as u32),
        )
    );
}

#[tokio::test]
async fn fail_with_invalid_income_token_account() {
    let (mut context, test_income_pool_market, test_income_pool, test_general_pool) = setup().await;

    let tx = Transaction::new_signed_with_payer(
        &[instruction::withdraw(
            &everlend_income_pools::id(),
            &test_general_pool.token_mint_pubkey,
            &test_income_pool_market.keypair.pubkey(),
            &test_income_pool.pool_pubkey,
            &Pubkey::new_unique(),
            &test_general_pool.pool_pubkey,
            &test_general_pool.token_account.pubkey(),
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
        TransactionError::InstructionError(0, InstructionError::InvalidArgument)
    );
}

#[tokio::test]
async fn fail_with_invalid_general_pool() {
    let (mut context, test_income_pool_market, test_income_pool, test_general_pool) = setup().await;

    let tx = Transaction::new_signed_with_payer(
        &[instruction::withdraw(
            &everlend_income_pools::id(),
            &test_general_pool.token_mint_pubkey,
            &test_income_pool_market.keypair.pubkey(),
            &test_income_pool.pool_pubkey,
            &test_income_pool.token_account.pubkey(),
            &Pubkey::new_unique(),
            &test_general_pool.token_account.pubkey(),
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
            InstructionError::Custom(EverlendError::InvalidAccountOwner as u32),
        )
    );
}

#[tokio::test]
async fn fail_with_invalid_general_pool_token_account() {
    let (mut context, test_income_pool_market, test_income_pool, test_general_pool) = setup().await;

    let tx = Transaction::new_signed_with_payer(
        &[instruction::withdraw(
            &everlend_income_pools::id(),
            &test_general_pool.token_mint_pubkey,
            &test_income_pool_market.keypair.pubkey(),
            &test_income_pool.pool_pubkey,
            &test_income_pool.token_account.pubkey(),
            &test_general_pool.pool_pubkey,
            &Pubkey::new_unique(),
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
        TransactionError::InstructionError(0, InstructionError::InvalidArgument)
    );
}
