#![cfg(feature = "test-bpf")]

use crate::utils::*;
use everlend_general_pool::instruction;
use everlend_utils::EverlendError;
use solana_program::instruction::InstructionError;
use solana_program_test::*;
use solana_sdk::{
    pubkey::Pubkey, signer::Signer, transaction::Transaction, transaction::TransactionError,
};
use everlend_registry::state::SetRegistryPoolConfigParams;
use spl_token::error::TokenError;

async fn setup() -> (
    ProgramTestContext,
    TestRegistry,
    TestGeneralPoolMarket,
    TestGeneralPool,
    TestGeneralPoolBorrowAuthority,
    LiquidityProvider,
) {
    let (mut context, _, _, registry) = presetup().await;

    let test_pool_market = TestGeneralPoolMarket::new();
    test_pool_market.init(&mut context, &registry.keypair.pubkey()).await.unwrap();

    let test_pool = TestGeneralPool::new(&test_pool_market, None);
    test_pool
        .create(&mut context, &test_pool_market)
        .await
        .unwrap();

    let test_pool_borrow_authority =
        TestGeneralPoolBorrowAuthority::new(&test_pool, context.payer.pubkey());
    test_pool_borrow_authority
        .create(
            &mut context,
            &test_pool_market,
            &test_pool,
            COLLATERAL_POOL_SHARE_ALLOWED,
        )
        .await
        .unwrap();
    registry
        .set_registry_pool_config(
            &mut context,
            &test_pool.pool_pubkey,
            SetRegistryPoolConfigParams { deposit_minimum: 0, withdraw_minimum: 0 }
        )
        .await
        .unwrap();

    let user = add_liquidity_provider(
        &mut context,
        &test_pool.token_mint_pubkey,
        &test_pool.pool_mint.pubkey(),
        101,
    )
    .await
    .unwrap();

    test_pool
        .deposit(&mut context, &registry, &test_pool_market, &user, 100)
        .await
        .unwrap();

    (
        context,
        registry,
        test_pool_market,
        test_pool,
        test_pool_borrow_authority,
        user,
    )
}

#[tokio::test]
async fn success() {
    let (mut context, _, test_pool_market, test_pool, test_pool_borrow_authority, user) =
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

    test_pool
        .repay(
            &mut context,
            &test_pool_market,
            &test_pool_borrow_authority,
            &user,
            amount_allowed,
            1,
        )
        .await
        .unwrap();

    assert_eq!(
        get_token_balance(&mut context, &user.token_account).await,
        0
    );
    assert_eq!(
        test_pool.get_data(&mut context).await.total_amount_borrowed,
        0
    );
}

#[tokio::test]
async fn fail_with_invalid_pool_market_pubkey_argument() {
    let (mut context, _, _test_pool_market, test_pool, test_pool_borrow_authority, user) =
        setup().await;

    let amount = 1;
    let interest_amount = 1;

    let tx = Transaction::new_signed_with_payer(
        &[instruction::repay(
            &everlend_general_pool::id(),
            // Wrong pool market pubkey
            &Pubkey::new_unique(),
            &test_pool.pool_pubkey,
            &test_pool_borrow_authority.pool_borrow_authority_pubkey,
            &user.token_account,
            &test_pool.token_account.pubkey(),
            &user.pubkey(),
            amount,
            interest_amount,
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
async fn fail_with_invalid_pool_pubkey_argument() {
    let (mut context, _, test_pool_market, test_pool, test_pool_borrow_authority, user) =
        setup().await;

    let amount = 1;
    let interest_amount = 1;

    let tx = Transaction::new_signed_with_payer(
        &[instruction::repay(
            &everlend_general_pool::id(),
            &test_pool_market.keypair.pubkey(),
            // Wrong pool pubkey
            &Pubkey::new_unique(),
            &test_pool_borrow_authority.pool_borrow_authority_pubkey,
            &user.token_account,
            &test_pool.token_account.pubkey(),
            &user.pubkey(),
            amount,
            interest_amount,
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
async fn fail_with_invalid_pool_borrow_authority_argument() {
    let (mut context, _, test_pool_market, test_pool, _test_pool_borrow_authority, user) =
        setup().await;

    let amount = 1;
    let interest_amount = 1;

    let tx = Transaction::new_signed_with_payer(
        &[instruction::repay(
            &everlend_general_pool::id(),
            &test_pool_market.keypair.pubkey(),
            &test_pool.pool_pubkey,
            // Wrong pubkey
            &Pubkey::new_unique(),
            &user.token_account,
            &test_pool.token_account.pubkey(),
            &user.pubkey(),
            amount,
            interest_amount,
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
async fn fail_with_invalid_pool_market() {
    let (mut context, registry, _test_pool_market, test_pool, test_pool_borrow_authority, user) =
        setup().await;

    let amount = 1;
    let interest_amount = 1;

    let test_pool_market = TestGeneralPoolMarket::new();
    test_pool_market.init(&mut context, &registry.keypair.pubkey()).await.unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[instruction::repay(
            &everlend_general_pool::id(),
            &test_pool_market.keypair.pubkey(),
            &test_pool.pool_pubkey,
            &test_pool_borrow_authority.pool_borrow_authority_pubkey,
            &user.token_account,
            &test_pool.token_account.pubkey(),
            &user.pubkey(),
            amount,
            interest_amount,
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
async fn fail_with_invalid_pool_token_account() {
    let (mut context, _, test_pool_market, test_pool, test_pool_borrow_authority, user) =
        setup().await;

    let amount = 1;
    let interest_amount = 1;

    let tx = Transaction::new_signed_with_payer(
        &[instruction::repay(
            &everlend_general_pool::id(),
            &test_pool_market.keypair.pubkey(),
            &test_pool.pool_pubkey,
            &test_pool_borrow_authority.pool_borrow_authority_pubkey,
            &user.token_account,
            &Pubkey::new_unique(),
            &user.pubkey(),
            amount,
            interest_amount,
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
async fn fail_with_invalid_repay_amount() {
    let (mut context, _, test_pool_market, test_pool, test_pool_borrow_authority, user) =
        setup().await;

    let amount = 1;
    let interest_amount = 1;

    let err = test_pool
        .repay(
            &mut context,
            &test_pool_market,
            &test_pool_borrow_authority,
            &user,
            amount,
            interest_amount,
        )
        .await
        .unwrap_err()
        .unwrap();

    assert_eq!(
        err,
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(EverlendError::RepayAmountCheckFailed as u32)
        )
    );
}

#[tokio::test]
async fn fail_with_invalid_source() {
    let (mut context, _, test_pool_market, test_pool, test_pool_borrow_authority, user) =
        setup().await;

    let amount = 1;
    let interest_amount = 1;

    let wrong_user = add_liquidity_provider(
        &mut context,
        &test_pool.token_mint_pubkey,
        &test_pool.pool_mint.pubkey(),
        101,
    )
    .await
    .unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[instruction::repay(
            &everlend_general_pool::id(),
            &test_pool_market.keypair.pubkey(),
            &test_pool.pool_pubkey,
            &test_pool_borrow_authority.pool_borrow_authority_pubkey,
            &wrong_user.token_account,
            &test_pool.token_account.pubkey(),
            &user.pubkey(),
            amount,
            interest_amount,
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
            InstructionError::Custom(TokenError::OwnerMismatch as u32)
        )
    );
}
