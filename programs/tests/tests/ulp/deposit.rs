#![cfg(feature = "test-bpf")]

use crate::utils::*;
use everlend_ulp::instruction;
use everlend_utils::EverlendError;
use solana_program::instruction::InstructionError;
use solana_program_test::*;
use solana_sdk::{
    pubkey::Pubkey, signer::Signer, transaction::Transaction, transaction::TransactionError,
};
use spl_token::error::TokenError;

async fn setup() -> (
    ProgramTestContext,
    TestPoolMarket,
    TestPool,
    LiquidityProvider,
) {
    let mut context = presetup().await.0;

    let test_pool_market = TestPoolMarket::new();
    test_pool_market.init(&mut context).await.unwrap();

    let test_pool = TestPool::new(&test_pool_market, None);
    test_pool
        .create(&mut context, &test_pool_market)
        .await
        .unwrap();

    let user = add_liquidity_provider(&mut context,  &test_pool.token_mint_pubkey, &test_pool.pool_mint.pubkey(), 9999 * EXP)
        .await
        .unwrap();

    (context, test_pool_market, test_pool, user)
}

#[tokio::test]
async fn success() {
    let (mut context, test_pool_market, test_pool, user) = setup().await;

    test_pool
        .deposit(&mut context, &test_pool_market, &user, 100)
        .await
        .unwrap();

    assert_eq!(
        get_token_balance(&mut context, &user.pool_account).await,
        100,
    );
}

#[tokio::test]
async fn success_with_rate() {
    let (mut context, test_pool_market, test_pool, user) = setup().await;
    let a = (100 * EXP, 50 * EXP, 100 * EXP); // Deposit -> Raise -> Deposit

    // 0. Deposit to 100
    test_pool
        .deposit(&mut context, &test_pool_market, &user, a.0)
        .await
        .unwrap();

    // 1. Raise total incoming token
    mint_tokens(
        &mut context,
        &test_pool.token_mint_pubkey,
        &test_pool.token_account.pubkey(),
        a.1,
    )
    .await
    .unwrap();

    // Update slot for next deposit
    context.warp_to_slot(3).unwrap();

    // 2. More deposit with changed rate
    test_pool
        .deposit(&mut context, &test_pool_market, &user, a.2)
        .await
        .unwrap();

    // Around 166
    let destination_amount = a.0 + (a.2 as u128 * a.0 as u128 / (a.0 + a.1) as u128) as u64;

    assert_eq!(
        get_token_balance(&mut context, &user.pool_account).await,
        destination_amount
    );
}

// Const amount for all fail tests with invalid arguments
const AMOUNT: u64 = 100 * EXP;

#[tokio::test]
async fn fail_with_invalid_pool_mint_pubkey_argument() {
    let (mut context, test_pool_market, test_pool, user) = setup().await;

    let tx = Transaction::new_signed_with_payer(
        &[instruction::deposit(
            &everlend_ulp::id(),
            &test_pool_market.keypair.pubkey(),
            &test_pool.pool_pubkey,
            &user.token_account,
            &user.pool_account,
            &test_pool.token_account.pubkey(),
            // Wrong pool mint pubkey
            &Pubkey::new_unique(),
            &user.pubkey(),
            AMOUNT,
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
async fn fail_with_invalid_token_account_pubkey_argument() {
    let (mut context, test_pool_market, test_pool, user) = setup().await;

    let tx = Transaction::new_signed_with_payer(
        &[instruction::deposit(
            &everlend_ulp::id(),
            &test_pool_market.keypair.pubkey(),
            &test_pool.pool_pubkey,
            &user.token_account,
            &user.pool_account,
            // Wrong token account pubkey
            &Pubkey::new_unique(),
            &test_pool.pool_mint.pubkey(),
            &user.pubkey(),
            AMOUNT,
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
async fn fail_with_invalid_destination_argument() {
    let (mut context, test_pool_market, test_pool, user) = setup().await;

    // Create new pool

    let tx = Transaction::new_signed_with_payer(
        &[instruction::deposit(
            &everlend_ulp::id(),
            &test_pool_market.keypair.pubkey(),
            &test_pool.pool_pubkey,
            &user.token_account,
            // Wrong destination
            &user.token_account,
            &test_pool.token_account.pubkey(),
            &test_pool.pool_mint.pubkey(),
            &user.pubkey(),
            AMOUNT,
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
            InstructionError::Custom(TokenError::MintMismatch as u32)
        )
    );
}

#[tokio::test]
async fn fail_with_invalid_source_argument() {
    let (mut context, test_pool_market, test_pool, user) = setup().await;

    let tx = Transaction::new_signed_with_payer(
        &[instruction::deposit(
            &everlend_ulp::id(),
            &test_pool_market.keypair.pubkey(),
            &test_pool.pool_pubkey,
            //Wrong source
            &user.pool_account,
            &user.pool_account,
            &test_pool.token_account.pubkey(),
            &test_pool.pool_mint.pubkey(),
            &user.pubkey(),
            AMOUNT,
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
            InstructionError::Custom(TokenError::InsufficientFunds as u32)
        )
    );
}

#[tokio::test]
async fn fail_with_invalid_pool_market_argument() {
    let (mut context, _test_pool_market, test_pool, user) = setup().await;

    let tx = Transaction::new_signed_with_payer(
        &[instruction::deposit(
            &everlend_ulp::id(),
            // Wrong pool market
            &Pubkey::new_unique(),
            &test_pool.pool_pubkey,
            &user.token_account,
            &user.pool_account,
            &test_pool.token_account.pubkey(),
            &test_pool.pool_mint.pubkey(),
            &user.pubkey(),
            AMOUNT,
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
            InstructionError::Custom(EverlendError::InvalidAccountOwner as u32)
        )
    );
}

#[tokio::test]
async fn fail_with_invalid_pool_argument() {
    let (mut context, test_pool_market, test_pool, user) = setup().await;

    let tx = Transaction::new_signed_with_payer(
        &[instruction::deposit(
            &everlend_ulp::id(),
            &test_pool_market.keypair.pubkey(),
            //Wrong pool
            &Pubkey::new_unique(),
            &user.token_account,
            &user.pool_account,
            &test_pool.token_account.pubkey(),
            &test_pool.pool_mint.pubkey(),
            &user.pubkey(),
            AMOUNT,
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
            InstructionError::Custom(EverlendError::InvalidAccountOwner as u32)
        )
    );
}
