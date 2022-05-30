#![cfg(feature = "test-bpf")]

use everlend_collateral_pool::instruction;
use everlend_utils::EverlendError;
use solana_program::instruction::InstructionError;
use solana_program_test::*;
use solana_sdk::{
    pubkey::Pubkey, signer::Signer, transaction::Transaction, transaction::TransactionError,
};

use crate::utils::{
    presetup,
    TestPoolMarket,
    TestPool,
    get_token_balance,
    EXP,
};
use crate::collateral_pool::collateral_pool_utils::{
    LiquidityProvider,
    add_liquidity_provider,
};

// Const amount for all fail tests with invalid arguments
const AMOUNT: u64 = 100 * EXP;

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

    let user = add_liquidity_provider(
        &mut context,
        &test_pool.token_mint_pubkey,
        9999 * EXP,
    )
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
        get_token_balance(&mut context, &test_pool.token_account.pubkey()).await,
        100,
    );
}

#[tokio::test]
async fn fail_with_invalid_token_account_pubkey_argument() {
    let (mut context, test_pool_market, test_pool, user) = setup().await;

    let tx = Transaction::new_signed_with_payer(
        &[instruction::deposit(
            &everlend_collateral_pool::id(),
            &test_pool_market.keypair.pubkey(),
            &test_pool.pool_pubkey,
            &user.token_account,
            // Wrong pool token account pubkey
            &Pubkey::new_unique(),
            &user.owner.pubkey(),
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
async fn fail_with_invalid_source_argument() {
    let (mut context, test_pool_market, test_pool, user) = setup().await;

    let tx = Transaction::new_signed_with_payer(
        &[instruction::deposit(
            &everlend_collateral_pool::id(),
            &test_pool_market.keypair.pubkey(),
            &test_pool.pool_pubkey,
            // Wrong source
            &Pubkey::new_unique(),
            &test_pool.token_account.pubkey(),
            &user.owner.pubkey(),
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
            InstructionError::InvalidAccountData
        )
    );
}

#[tokio::test]
async fn fail_with_invalid_pool_market_argument() {
    let (mut context, _test_pool_market, test_pool, user) = setup().await;

    let tx = Transaction::new_signed_with_payer(
        &[instruction::deposit(
            &everlend_collateral_pool::id(),
            // Wrong pool market
            &Pubkey::new_unique(),
            &test_pool.pool_pubkey,
            &user.token_account,
            &test_pool.token_account.pubkey(),
            &user.owner.pubkey(),
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
            &everlend_collateral_pool::id(),
            &test_pool_market.keypair.pubkey(),
            //Wrong pool
            &Pubkey::new_unique(),
            &user.token_account,
            &test_pool.token_account.pubkey(),
            &user.owner.pubkey(),
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
