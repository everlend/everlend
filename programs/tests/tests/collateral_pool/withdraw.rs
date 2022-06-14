#![cfg(feature = "test-bpf")]

use everlend_collateral_pool::instruction;
use everlend_utils::EverlendError;
use solana_program::instruction::InstructionError;
use solana_program_test::*;
use solana_sdk::{
    pubkey::Pubkey, signer::Signer, transaction::Transaction, transaction::TransactionError 
};

use crate::utils::{
    presetup,
    TestPoolMarket,
    TestPool,
    TestPoolWithdrawAuthority,
    get_token_balance,
    EXP,
};
use crate::utils::collateral_pool_liquidity_provider::{LiquidityProvider};

// Const amount for all fail tests with invalid arguments
const AMOUNT: u64 = 100 * EXP;
const INITIAL_AMOUNT: u64 = 1000;
const DEPOSIT_AMOUNT: u64 = 100;
const WITHDRAW_AMOUNT: u64 = 50;

async fn setup() -> (
    ProgramTestContext,
    TestPoolMarket,
    TestPool,
    TestPoolWithdrawAuthority,
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
    let withdraw_authority_pubkey = context.payer.pubkey();
    let withdraw_authority = TestPoolWithdrawAuthority::new(&test_pool, &withdraw_authority_pubkey);
    withdraw_authority
        .create(&mut context, &test_pool_market, &test_pool, &withdraw_authority_pubkey)
        .await
        .unwrap();

    let user = LiquidityProvider::new(
        &mut context,
        &test_pool.token_mint_pubkey,
        INITIAL_AMOUNT,
    )
    .await;

    (context, test_pool_market, test_pool, withdraw_authority, user)
}

#[tokio::test]
async fn success() {
    let (mut context, test_pool_market, test_pool, withdraw_authority, user) = setup().await;

    test_pool
        .deposit(&mut context, &test_pool_market, &user, DEPOSIT_AMOUNT)
        .await
        .unwrap();
    test_pool
        .withdraw(&mut context, &test_pool_market, &withdraw_authority, None, &user, WITHDRAW_AMOUNT)
        .await
        .unwrap();

    assert_eq!(
        get_token_balance(&mut context, &user.token_account).await,
        INITIAL_AMOUNT - DEPOSIT_AMOUNT + WITHDRAW_AMOUNT,
    );
    assert_eq!(
        get_token_balance(&mut context, &test_pool.token_account.pubkey()).await,
        DEPOSIT_AMOUNT - WITHDRAW_AMOUNT, 
    );
}

#[tokio::test]
async fn fail_with_invalid_token_account_pubkey_argument() {
    let (mut context, test_pool_market, test_pool, withdraw_authority, user) = setup().await;

    let tx = Transaction::new_signed_with_payer(
        &[instruction::withdraw(
            &everlend_collateral_pool::id(),
            &test_pool_market.keypair.pubkey(),
            &test_pool.pool_pubkey,
            &withdraw_authority.pool_withdraw_authority_pubkey,
            &user.token_account,
            // Wrong token account pubkey
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
async fn fail_invalid_destination_argument() {
    let (mut context, test_pool_market, test_pool, withdraw_authority, user) = setup().await;

    // 0. Deposit to 100
    test_pool
        .deposit(&mut context, &test_pool_market, &user, 100)
        .await
        .unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[instruction::withdraw(
            &everlend_collateral_pool::id(),
            &test_pool_market.keypair.pubkey(),
            &test_pool.pool_pubkey,
            &withdraw_authority.pool_withdraw_authority_pubkey,
            // wrong destination
            &Pubkey::new_unique(),
            &test_pool.token_account.pubkey(),
            &user.owner.pubkey(),
            50,
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
            InstructionError::InvalidArgument
        )
    );
}

#[tokio::test]
async fn fail_withdraw_from_empty_pool_mint() {
    let (mut context, test_pool_market, test_pool, withdraw_authority, user) = setup().await;

    let tx = Transaction::new_signed_with_payer(
        &[instruction::withdraw(
            &everlend_collateral_pool::id(),
            &test_pool_market.keypair.pubkey(),
            &test_pool.pool_pubkey,
            &withdraw_authority.pool_withdraw_authority_pubkey,
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
        TransactionError::InstructionError(0, InstructionError::InvalidArgument)
    );
}

#[tokio::test]
async fn fail_with_invalid_pool_market_argument() {
    let (mut context, _test_pool_market, test_pool, withdraw_authority, user) = setup().await;

    let tx = Transaction::new_signed_with_payer(
        &[instruction::withdraw(
            &everlend_collateral_pool::id(),
            // Wrong pool market
            &Pubkey::new_unique(),
            &test_pool.pool_pubkey,
            &withdraw_authority.pool_withdraw_authority_pubkey,
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
    let (mut context, test_pool_market, test_pool, withdraw_authority, user) = setup().await;

    let tx = Transaction::new_signed_with_payer(
        &[instruction::withdraw(
            &everlend_collateral_pool::id(),
            &test_pool_market.keypair.pubkey(),
            //Wrong pool
            &Pubkey::new_unique(),
            &withdraw_authority.pool_withdraw_authority_pubkey,
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
