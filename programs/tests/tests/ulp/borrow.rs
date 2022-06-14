#![cfg(feature = "test-bpf")]

use everlend_ulp::instruction;
use everlend_utils::EverlendError;
use solana_program::instruction::InstructionError;
use solana_program_test::*;
use solana_sdk::{
    pubkey::Pubkey, signature::Keypair, signer::Signer, transaction::Transaction,
    transaction::TransactionError,
};
use spl_token::error::TokenError;
use crate::utils::{
    presetup,
    UlpMarket,
    UniversalLiquidityPool,
    UniversalLiquidityPoolBorrowAuthority,
    add_liquidity_provider,
    get_token_balance,
    LiquidityProvider,
};
pub const ULP_SHARE_ALLOWED: u16 = 10_000;

async fn setup() -> (
    ProgramTestContext,
    UlpMarket,
    UniversalLiquidityPool,
    UniversalLiquidityPoolBorrowAuthority,
    LiquidityProvider,
) {
    let mut env = presetup().await;

    let test_pool_market = UlpMarket::new();
    test_pool_market.init(&mut env.context).await.unwrap();

    let test_pool = UniversalLiquidityPool::new(&test_pool_market, None);
    test_pool
        .create(&mut env.context, &test_pool_market)
        .await
        .unwrap();

    let test_pool_borrow_authority =
        UniversalLiquidityPoolBorrowAuthority::new(&test_pool, env.context.payer.pubkey());
    test_pool_borrow_authority
        .create(
            &mut env.context,
            &test_pool_market,
            &test_pool,
            ULP_SHARE_ALLOWED,
        )
        .await
        .unwrap();

    let user = add_liquidity_provider(
        &mut env.context,
        &test_pool.token_mint_pubkey,
        &test_pool.pool_mint.pubkey(),
        100,
    )
    .await
    .unwrap();

    test_pool
        .deposit(&mut env.context, &test_pool_market, &user, 100)
        .await
        .unwrap();

    (
        env.context,
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
                None,
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

    let token_amount = get_token_balance(&mut context, &test_pool.token_account.pubkey()).await;
    let total_amount_borrowed = test_pool.get_data(&mut context).await.total_amount_borrowed;
    let total_pool_amount = token_amount + total_amount_borrowed;

    let amount_allowed = test_pool_borrow_authority
        .get_data(&mut context)
        .await
        .get_amount_allowed(total_pool_amount)
        .unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[instruction::borrow(
            &everlend_ulp::id(),
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
    let token_amount = get_token_balance(&mut context, &test_pool.token_account.pubkey()).await;
    let total_amount_borrowed = test_pool.get_data(&mut context).await.total_amount_borrowed;
    let total_pool_amount = token_amount + total_amount_borrowed;

    let amount_allowed = test_pool_borrow_authority
        .get_data(&mut context)
        .await
        .get_amount_allowed(total_pool_amount)
        .unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[instruction::borrow(
            &everlend_ulp::id(),
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
    let token_amount = get_token_balance(&mut context, &test_pool.token_account.pubkey()).await;
    let total_amount_borrowed = test_pool.get_data(&mut context).await.total_amount_borrowed;
    let total_pool_amount = token_amount + total_amount_borrowed;

    let amount_allowed = test_pool_borrow_authority
        .get_data(&mut context)
        .await
        .get_amount_allowed(total_pool_amount)
        .unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[instruction::borrow(
            &everlend_ulp::id(),
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
    let token_amount = get_token_balance(&mut context, &test_pool.token_account.pubkey()).await;
    let total_amount_borrowed = test_pool.get_data(&mut context).await.total_amount_borrowed;
    let total_pool_amount = token_amount + total_amount_borrowed;

    let amount_allowed = test_pool_borrow_authority
        .get_data(&mut context)
        .await
        .get_amount_allowed(total_pool_amount)
        .unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[instruction::borrow(
            &everlend_ulp::id(),
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
