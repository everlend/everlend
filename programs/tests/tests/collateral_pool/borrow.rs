#![cfg(feature = "test-bpf")]

use everlend_collateral_pool::instruction;
use everlend_utils::EverlendError;
use solana_program::instruction::InstructionError;
use solana_program_test::*;
use solana_sdk::{
    pubkey::Pubkey, signature::Keypair, signer::Signer, transaction::Transaction,
    transaction::TransactionError,
};
use crate::utils::{
    presetup,
    TestPoolMarket,
    TestPool,
    TestPoolBorrowAuthority,
    get_token_balance,
    get_amount_allowed,
    COLLATERAL_POOL_SHARE_ALLOWED,
};
use crate::utils::collateral_pool_liquidity_provider::{LiquidityProvider};

async fn setup() -> (
    ProgramTestContext,
    TestPoolMarket,
    TestPool,
    TestPoolBorrowAuthority,
    LiquidityProvider,
) {
    let mut context = presetup().await.context;

    let test_pool_market = TestPoolMarket::new();
    test_pool_market.init(&mut context).await.unwrap();
    let test_pool = TestPool::new(&test_pool_market, None);
    test_pool
        .create(&mut context, &test_pool_market)
        .await
        .unwrap();

    let test_pool_borrow_authority =
        TestPoolBorrowAuthority::new(&test_pool, context.payer.pubkey());
    test_pool_borrow_authority
        .create(
            &mut context,
            &test_pool_market,
            &test_pool,
            COLLATERAL_POOL_SHARE_ALLOWED,
        )
        .await
        .unwrap();

    let user = LiquidityProvider::new(
        &mut context,
        &test_pool.token_mint_pubkey,
        100,
    )
    .await;

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
            None,
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
                Some(&Keypair::new()),
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
    let (mut context, test_pool_market, test_pool, test_pool_borrow_authority, _) =
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
                None,
                &Pubkey::new_unique(),
                amount_allowed,
            )
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(
            0,
            InstructionError::InvalidAccountData
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
            &everlend_collateral_pool::id(),
            &test_pool_market.keypair.pubkey(),
            &test_pool.pool_pubkey,
            &test_pool_borrow_authority.pool_borrow_authority_pubkey,
            &user.token_account,
            &Pubkey::new_unique(),
            &context.payer.pubkey(),
            amount_allowed,
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
async fn fail_invalid_pool_market() {
    let (mut context, _test_pool_market, test_pool, test_pool_borrow_authority, user) =
        setup().await;
    let amount_allowed =
        get_amount_allowed(&mut context, &test_pool, &test_pool_borrow_authority).await;

    let tx = Transaction::new_signed_with_payer(
        &[instruction::borrow(
            &everlend_collateral_pool::id(),
            &Pubkey::new_unique(),
            &test_pool.pool_pubkey,
            &test_pool_borrow_authority.pool_borrow_authority_pubkey,
            &user.token_account,
            &test_pool.token_account.pubkey(),
            &context.payer.pubkey(),
            amount_allowed,
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
            InstructionError::Custom(EverlendError::InvalidAccountOwner as u32)
        )
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
            &everlend_collateral_pool::id(),
            &test_pool_market.keypair.pubkey(),
            &Pubkey::new_unique(),
            &test_pool_borrow_authority.pool_borrow_authority_pubkey,
            &user.token_account,
            &test_pool.token_account.pubkey(),
            &context.payer.pubkey(),
            amount_allowed,
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
            InstructionError::Custom(EverlendError::InvalidAccountOwner as u32)
        )
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
            &everlend_collateral_pool::id(),
            &test_pool_market.keypair.pubkey(),
            &test_pool.pool_pubkey,
            &Pubkey::new_unique(),
            &user.token_account,
            &test_pool.token_account.pubkey(),
            &context.payer.pubkey(),
            amount_allowed,
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
            InstructionError::Custom(EverlendError::InvalidAccountOwner as u32)
        )
    );
}
