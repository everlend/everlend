#![cfg(feature = "test-bpf")]

use solana_program::instruction::InstructionError;
use solana_program::pubkey::Pubkey;
use solana_program_test::*;
use solana_sdk::signer::Signer;
use solana_sdk::transaction::{Transaction, TransactionError};
use spl_token::error::TokenError;

use everlend_income_pools::instruction;
use everlend_utils::EverlendError;

use crate::utils::*;

async fn setup() -> (
    ProgramTestContext,
    TestGeneralPoolMarket,
    TestIncomePoolMarket,
    TestIncomePool,
    TokenHolder,
) {
    let (mut context, _, _, registry) = presetup().await;

    let general_pool_market = TestGeneralPoolMarket::new();
    general_pool_market.init(&mut context, &registry.keypair.pubkey()).await.unwrap();

    let test_income_pool_market = TestIncomePoolMarket::new();
    test_income_pool_market
        .init(&mut context, &general_pool_market)
        .await
        .unwrap();

    let test_income_pool = TestIncomePool::new(&test_income_pool_market, None);
    test_income_pool
        .create(&mut context, &test_income_pool_market)
        .await
        .unwrap();

    let user = add_token_holder(
        &mut context,
        &test_income_pool.token_mint_pubkey,
        9999 * EXP,
    )
    .await
    .unwrap();

    (
        context,
        general_pool_market,
        test_income_pool_market,
        test_income_pool,
        user,
    )
}

#[tokio::test]
async fn success() {
    let (mut context, _, test_income_pool_market, test_income_pool, user) = setup().await;

    test_income_pool
        .deposit(&mut context, &test_income_pool_market, &user, 100)
        .await
        .unwrap();

    assert_eq!(
        get_token_balance(&mut context, &test_income_pool.token_account.pubkey()).await,
        100
    );
}

#[tokio::test]
async fn fail_with_invalid_income_pool_market() {
    let (mut context, _, _, test_income_pool, user) = setup().await;

    let amount = 100;

    let tx = Transaction::new_signed_with_payer(
        &[instruction::deposit(
            &everlend_income_pools::id(),
            &Pubkey::new_unique(),
            &test_income_pool.pool_pubkey,
            &user.token_account,
            &test_income_pool.token_account.pubkey(),
            &user.pubkey(),
            amount,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &user.owner],
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
    let (mut context, _, test_income_pool_market, test_income_pool, user) = setup().await;

    let amount = 100;

    let tx = Transaction::new_signed_with_payer(
        &[instruction::deposit(
            &everlend_income_pools::id(),
            &test_income_pool_market.keypair.pubkey(),
            &Pubkey::new_unique(),
            &user.token_account,
            &test_income_pool.token_account.pubkey(),
            &user.pubkey(),
            amount,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &user.owner],
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
async fn fail_with_wrong_income_pool() {
    let (mut context, test_general_pool_market, test_income_pool_market, test_income_pool, user) =
        setup().await;

    let amount = 100;

    let wrong_income_pool = {
        let test_income_pool_market = TestIncomePoolMarket::new();
        test_income_pool_market
            .init(&mut context, &test_general_pool_market)
            .await
            .unwrap();

        let test_income_pool = TestIncomePool::new(&test_income_pool_market, None);
        test_income_pool
            .create(&mut context, &test_income_pool_market)
            .await
            .unwrap();
        test_income_pool
    };

    let tx = Transaction::new_signed_with_payer(
        &[instruction::deposit(
            &everlend_income_pools::id(),
            &test_income_pool_market.keypair.pubkey(),
            &wrong_income_pool.pool_pubkey,
            &user.token_account,
            &test_income_pool.token_account.pubkey(),
            &user.pubkey(),
            amount,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &user.owner],
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
async fn fail_with_invalid_token_account() {
    let (mut context, _, test_income_pool_market, test_income_pool, user) = setup().await;

    let amount = 100;

    let tx = Transaction::new_signed_with_payer(
        &[instruction::deposit(
            &everlend_income_pools::id(),
            &test_income_pool_market.keypair.pubkey(),
            &test_income_pool.pool_pubkey,
            &user.token_account,
            &Pubkey::new_unique(),
            &user.pubkey(),
            amount,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &user.owner],
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
async fn fail_with_invalid_user_transfer_authority() {
    let (mut context, _, test_income_pool_market, test_income_pool, user) = setup().await;

    let amount = 100;

    let invalid_user = add_token_holder(
        &mut context,
        &test_income_pool.token_mint_pubkey,
        9999 * EXP,
    )
    .await
    .unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[instruction::deposit(
            &everlend_income_pools::id(),
            &test_income_pool_market.keypair.pubkey(),
            &test_income_pool.pool_pubkey,
            &user.token_account,
            &test_income_pool.token_account.pubkey(),
            &invalid_user.pubkey(),
            amount,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &invalid_user.owner],
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
            InstructionError::Custom(TokenError::OwnerMismatch as u32),
        )
    );
}
