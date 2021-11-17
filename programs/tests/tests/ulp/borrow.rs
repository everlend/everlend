#![cfg(feature = "test-bpf")]

use crate::utils::*;
use solana_program::instruction::InstructionError;
use solana_program_test::*;
use solana_sdk::{
    pubkey::Pubkey, signature::Keypair, signer::Signer, transaction::Transaction,
    transaction::TransactionError,
};
use everlend_ulp::{id, instruction};
use spl_token::error::TokenError;

async fn setup() -> (
    ProgramTestContext,
    TestPoolMarket,
    TestPool,
    TestPoolBorrowAuthority,
    LiquidityProvider,
) {
    let mut context = program_test().start_with_context().await;

    let test_pool_market = TestPoolMarket::new();
    test_pool_market.init(&mut context).await.unwrap();

    let test_pool = TestPool::new(&test_pool_market);
    test_pool
        .create(&mut context, &test_pool_market)
        .await
        .unwrap();

    let test_pool_borrow_authority = TestPoolBorrowAuthority::new(&test_pool, None);
    test_pool_borrow_authority
        .create(&mut context, &test_pool_market, &test_pool, SHARE_ALLOWED)
        .await
        .unwrap();

    let user = add_liquidity_provider(&mut context, &test_pool, 100)
        .await
        .unwrap();

    test_pool
        .deposit(&mut context, &test_pool_market, &user, 100)
        .await
        .unwrap();

    (
        context,
        test_pool_market,
        test_pool,
        test_pool_borrow_authority,
        user,
    )
}

#[tokio::test]
async fn success() {
    let (mut context, test_pool_market, test_pool, test_pool_borrow_authority, user) =
        setup().await;
    let amount_allowed = test_pool_borrow_authority
        .get_amount_allowed(&mut context)
        .await;

    test_pool
        .borrow(
            &mut context,
            &test_pool_market,
            &test_pool_borrow_authority,
            &test_pool_borrow_authority.borrow_authority,
            &user.token_account,
            amount_allowed,
        )
        .await
        .unwrap();

    assert_eq!(
        get_token_balance(&mut context, &user.token_account).await,
        amount_allowed
    );
    assert_eq!(
        test_pool.get_data(&mut context).await.total_amount_borrowed,
        amount_allowed
    );
}

#[tokio::test]
async fn fail_wrong_borrow_authority() {
    let (mut context, test_pool_market, test_pool, test_pool_borrow_authority, user) =
        setup().await;
    let amount_allowed = test_pool_borrow_authority
        .get_amount_allowed(&mut context)
        .await;

    assert_eq!(
        test_pool
            .borrow(
                &mut context,
                &test_pool_market,
                &test_pool_borrow_authority,
                &Keypair::new(),
                &user.token_account,
                amount_allowed,
            )
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(0, InstructionError::InvalidArgument)
    );
}

#[tokio::test]
async fn fail_invalid_destination() {
    let (mut context, test_pool_market, test_pool, test_pool_borrow_authority, user) =
        setup().await;
    let amount_allowed = test_pool_borrow_authority
        .get_amount_allowed(&mut context)
        .await;

    assert_eq!(
        test_pool
            .borrow(
                &mut context,
                &test_pool_market,
                &test_pool_borrow_authority,
                &test_pool_borrow_authority.borrow_authority,
                &user.pool_account,
                amount_allowed,
            )
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
async fn fail_invalid_token_account() {
    let (mut context, test_pool_market, test_pool, test_pool_borrow_authority, user) =
        setup().await;
    let amount_allowed =
        get_amount_allowed(&mut context, &test_pool, &test_pool_borrow_authority).await;

    let tx = Transaction::new_signed_with_payer(
        &[instruction::borrow(
            &id(),
            &test_pool_market.pool_market.pubkey(),
            &test_pool.pool_pubkey,
            &test_pool_borrow_authority.pool_borrow_authority_pubkey,
            &user.token_account,
            &Pubkey::new_unique(),
            &test_pool_borrow_authority.borrow_authority.pubkey(),
            amount_allowed,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &test_pool_borrow_authority.borrow_authority],
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
async fn fail_invalid_pool_market() {
    let (mut context, _test_pool_market, test_pool, test_pool_borrow_authority, user) =
        setup().await;
    let amount_allowed =
        get_amount_allowed(&mut context, &test_pool, &test_pool_borrow_authority).await;

    let tx = Transaction::new_signed_with_payer(
        &[instruction::borrow(
            &id(),
            &Pubkey::new_unique(),
            &test_pool.pool_pubkey,
            &test_pool_borrow_authority.pool_borrow_authority_pubkey,
            &user.token_account,
            &test_pool.token_account.pubkey(),
            &test_pool_borrow_authority.borrow_authority.pubkey(),
            amount_allowed,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &test_pool_borrow_authority.borrow_authority],
        context.last_blockhash,
    );

    assert_eq!(
        context
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(0, InstructionError::IncorrectProgramId)
    );
}

#[tokio::test]
async fn fail_invalid_pool() {
    let (mut context, test_pool_market, test_pool, test_pool_borrow_authority, user) =
        setup().await;
    let amount_allowed =
        get_amount_allowed(&mut context, &test_pool, &test_pool_borrow_authority).await;

    let tx = Transaction::new_signed_with_payer(
        &[instruction::borrow(
            &id(),
            &test_pool_market.pool_market.pubkey(),
            &Pubkey::new_unique(),
            &test_pool_borrow_authority.pool_borrow_authority_pubkey,
            &user.token_account,
            &test_pool.token_account.pubkey(),
            &test_pool_borrow_authority.borrow_authority.pubkey(),
            amount_allowed,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &test_pool_borrow_authority.borrow_authority],
        context.last_blockhash,
    );

    assert_eq!(
        context
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(0, InstructionError::IncorrectProgramId)
    );
}

#[tokio::test]
async fn fail_invalid_pool_borrow_authority() {
    let (mut context, test_pool_market, test_pool, test_pool_borrow_authority, user) =
        setup().await;
    let amount_allowed =
        get_amount_allowed(&mut context, &test_pool, &test_pool_borrow_authority).await;

    let tx = Transaction::new_signed_with_payer(
        &[instruction::borrow(
            &id(),
            &test_pool_market.pool_market.pubkey(),
            &test_pool.pool_pubkey,
            &Pubkey::new_unique(),
            &user.token_account,
            &test_pool.token_account.pubkey(),
            &test_pool_borrow_authority.borrow_authority.pubkey(),
            amount_allowed,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &test_pool_borrow_authority.borrow_authority],
        context.last_blockhash,
    );

    assert_eq!(
        context
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(0, InstructionError::IncorrectProgramId)
    );
}