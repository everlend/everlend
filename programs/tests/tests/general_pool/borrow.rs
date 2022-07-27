use crate::utils::*;
use everlend_general_pool::instruction;
use everlend_registry::state::SetRegistryPoolConfigParams;
use everlend_utils::EverlendError;
use solana_program::instruction::InstructionError;
use solana_program_test::*;
use solana_sdk::{
    pubkey::Pubkey, signature::Keypair, signer::Signer, transaction::Transaction,
    transaction::TransactionError,
};
use spl_token::error::TokenError;

async fn setup() -> (
    ProgramTestContext,
    TestGeneralPoolMarket,
    TestGeneralPool,
    TestGeneralPoolBorrowAuthority,
    LiquidityProvider,
) {
    let mut env = presetup().await;

    let test_pool_market = TestGeneralPoolMarket::new();
    test_pool_market
        .init(&mut env.context, &env.registry.keypair.pubkey())
        .await
        .unwrap();

    let test_pool = TestGeneralPool::new(&test_pool_market, None);
    test_pool
        .create(&mut env.context, &test_pool_market)
        .await
        .unwrap();

    let test_pool_borrow_authority =
        TestGeneralPoolBorrowAuthority::new(&test_pool, env.context.payer.pubkey());
    test_pool_borrow_authority
        .create(
            &mut env.context,
            &test_pool_market,
            &test_pool,
            GENERAL_POOL_SHARE_ALLOWED,
        )
        .await
        .unwrap();
    env.registry
        .set_registry_pool_config(
            &mut env.context,
            &test_pool.pool_pubkey,
            SetRegistryPoolConfigParams {
                deposit_minimum: 0,
                withdraw_minimum: 0,
            },
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

    let mining_acc = test_pool
        .init_user_mining(&mut env.context, &test_pool_market, &user)
        .await;
    test_pool
        .deposit(
            &mut env.context,
            &env.registry,
            &test_pool_market,
            &user,
            mining_acc,
            100,
        )
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
    let amount_allowed =
        get_amount_allowed_general(&mut context, &test_pool, &test_pool_borrow_authority).await;

    let tx = Transaction::new_signed_with_payer(
        &[instruction::borrow(
            &everlend_general_pool::id(),
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
        get_amount_allowed_general(&mut context, &test_pool, &test_pool_borrow_authority).await;

    let tx = Transaction::new_signed_with_payer(
        &[instruction::borrow(
            &everlend_general_pool::id(),
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
        get_amount_allowed_general(&mut context, &test_pool, &test_pool_borrow_authority).await;

    let tx = Transaction::new_signed_with_payer(
        &[instruction::borrow(
            &everlend_general_pool::id(),
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
        get_amount_allowed_general(&mut context, &test_pool, &test_pool_borrow_authority).await;

    let tx = Transaction::new_signed_with_payer(
        &[instruction::borrow(
            &everlend_general_pool::id(),
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
