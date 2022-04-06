#![cfg(feature = "test-bpf")]

use crate::utils::*;
use everlend_general_pool::{find_transit_program_address, instruction};
use everlend_utils::EverlendError;
use solana_program::clock::Slot;
use solana_program::instruction::InstructionError;
use solana_program::pubkey::Pubkey;
use solana_program_test::*;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use solana_sdk::transaction::{Transaction, TransactionError};

const WARP_SLOT: Slot = 3;

async fn setup() -> (
    ProgramTestContext,
    TestGeneralPoolMarket,
    TestGeneralPool,
    TestGeneralPoolBorrowAuthority,
    LiquidityProvider,
) {
    let mut context = presetup().await.0;

    let test_pool_market = TestGeneralPoolMarket::new();
    test_pool_market.init(&mut context).await.unwrap();

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
            ULP_SHARE_ALLOWED,
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

    transfer(&mut context, &user.owner.pubkey(), 5000000)
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
    let (mut context, test_pool_market, test_pool, _pool_borrow_authority, user) = setup().await;

    test_pool
        .deposit(&mut context, &test_pool_market, &user, 100)
        .await
        .unwrap();

    context.warp_to_slot(WARP_SLOT + 5).unwrap();

    test_pool
        .withdraw_request(&mut context, &test_pool_market, &user, 50)
        .await
        .unwrap();

    let (withdraw_requests_pubkey, withdraw_requests) = test_pool
        .get_withdrawal_requests(&mut context, &test_pool_market)
        .await;
    let (transit_account, _) = find_transit_program_address(
        &everlend_general_pool::id(),
        &test_pool_market.keypair.pubkey(),
        &test_pool.pool_mint.pubkey(),
    );
    let withdraw_request = test_pool
        .get_withdrawal_request(&mut context, &withdraw_requests_pubkey, &user.pubkey())
        .await;

    assert_eq!(
        get_token_balance(&mut context, &user.pool_account).await,
        50
    );
    assert_eq!(get_token_balance(&mut context, &transit_account).await, 50);
    assert_eq!(withdraw_requests.liquidity_supply, 50);
    assert_eq!(withdraw_requests.next_ticket, 1);
    assert_eq!(withdraw_request.liquidity_amount, 50);
}

#[tokio::test]
async fn fail_with_invalid_pool_market() {
    let (mut context, test_pool_market, test_pool, _pool_borrow_authority, user) = setup().await;

    test_pool
        .deposit(&mut context, &test_pool_market, &user, 100)
        .await
        .unwrap();

    context.warp_to_slot(WARP_SLOT + 5).unwrap();

    let withdraw_amount = 50;

    let tx = Transaction::new_signed_with_payer(
        &[instruction::withdraw_request(
            &everlend_general_pool::id(),
            &Pubkey::new_unique(),
            &test_pool.pool_pubkey,
            &user.pool_account,
            &user.token_account,
            &test_pool.token_account.pubkey(),
            &test_pool.token_mint_pubkey,
            &test_pool.pool_mint.pubkey(),
            &user.pubkey(),
            withdraw_amount,
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
    )
}

#[tokio::test]
async fn fail_with_invalid_pool() {
    let (mut context, test_pool_market, test_pool, _pool_borrow_authority, user) = setup().await;

    test_pool
        .deposit(&mut context, &test_pool_market, &user, 100)
        .await
        .unwrap();

    context.warp_to_slot(WARP_SLOT + 5).unwrap();

    let withdraw_amount = 50;

    let tx = Transaction::new_signed_with_payer(
        &[instruction::withdraw_request(
            &everlend_general_pool::id(),
            &test_pool_market.keypair.pubkey(),
            &Pubkey::new_unique(),
            &user.pool_account,
            &user.token_account,
            &test_pool.token_account.pubkey(),
            &test_pool.token_mint_pubkey,
            &test_pool.pool_mint.pubkey(),
            &user.pubkey(),
            withdraw_amount,
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
    )
}

#[tokio::test]
async fn fail_with_invalid_destination() {
    let (mut context, test_pool_market, test_pool, _pool_borrow_authority, user) = setup().await;

    test_pool
        .deposit(&mut context, &test_pool_market, &user, 100)
        .await
        .unwrap();

    context.warp_to_slot(WARP_SLOT + 5).unwrap();

    let withdraw_amount = 50;

    let tx = Transaction::new_signed_with_payer(
        &[instruction::withdraw_request(
            &everlend_general_pool::id(),
            &test_pool_market.keypair.pubkey(),
            &test_pool.pool_pubkey,
            &user.pool_account,
            &Pubkey::new_unique(),
            &test_pool.token_account.pubkey(),
            &test_pool.token_mint_pubkey,
            &test_pool.pool_mint.pubkey(),
            &user.pubkey(),
            withdraw_amount,
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
        TransactionError::InstructionError(0, InstructionError::InvalidAccountData)
    )
}

#[tokio::test]
async fn fail_with_invalid_token_account() {
    let (mut context, test_pool_market, test_pool, _pool_borrow_authority, user) = setup().await;

    test_pool
        .deposit(&mut context, &test_pool_market, &user, 100)
        .await
        .unwrap();

    context.warp_to_slot(WARP_SLOT + 5).unwrap();

    let withdraw_amount = 50;

    let tx = Transaction::new_signed_with_payer(
        &[instruction::withdraw_request(
            &everlend_general_pool::id(),
            &test_pool_market.keypair.pubkey(),
            &test_pool.pool_pubkey,
            &user.pool_account,
            &user.token_account,
            &Pubkey::new_unique(),
            &test_pool.token_mint_pubkey,
            &test_pool.pool_mint.pubkey(),
            &user.pubkey(),
            withdraw_amount,
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
    )
}

#[tokio::test]
async fn fail_with_invalid_token_mint() {
    let (mut context, test_pool_market, test_pool, _pool_borrow_authority, user) = setup().await;

    test_pool
        .deposit(&mut context, &test_pool_market, &user, 100)
        .await
        .unwrap();

    context.warp_to_slot(WARP_SLOT + 5).unwrap();

    let withdraw_amount = 50;

    let tx = Transaction::new_signed_with_payer(
        &[instruction::withdraw_request(
            &everlend_general_pool::id(),
            &test_pool_market.keypair.pubkey(),
            &test_pool.pool_pubkey,
            &user.pool_account,
            &user.token_account,
            &test_pool.token_account.pubkey(),
            &Pubkey::new_unique(),
            &test_pool.pool_mint.pubkey(),
            &user.pubkey(),
            withdraw_amount,
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
    )
}

#[tokio::test]
async fn fail_with_invalid_pool_mint() {
    let (mut context, test_pool_market, test_pool, _pool_borrow_authority, user) = setup().await;

    test_pool
        .deposit(&mut context, &test_pool_market, &user, 100)
        .await
        .unwrap();

    context.warp_to_slot(WARP_SLOT + 5).unwrap();

    let withdraw_amount = 50;

    let tx = Transaction::new_signed_with_payer(
        &[instruction::withdraw_request(
            &everlend_general_pool::id(),
            &test_pool_market.keypair.pubkey(),
            &test_pool.pool_pubkey,
            &user.pool_account,
            &user.token_account,
            &test_pool.token_account.pubkey(),
            &test_pool.token_mint_pubkey,
            &Pubkey::new_unique(),
            &user.pubkey(),
            withdraw_amount,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &user.owner],
        context.last_blockhash,
    );

    assert_eq!(
        dbg!(context
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err())
        .unwrap(),
        TransactionError::InstructionError(0, InstructionError::InvalidArgument)
    )
}

#[tokio::test]
async fn fail_with_wrong_user_transfer_authority() {
    let (mut context, test_pool_market, test_pool, _pool_borrow_authority, user) = setup().await;

    test_pool
        .deposit(&mut context, &test_pool_market, &user, 100)
        .await
        .unwrap();

    context.warp_to_slot(WARP_SLOT + 5).unwrap();

    let withdraw_amount = 50;

    let wrong_user_authority = Keypair::new();

    let tx = Transaction::new_signed_with_payer(
        &[instruction::withdraw_request(
            &everlend_general_pool::id(),
            &test_pool_market.keypair.pubkey(),
            &test_pool.pool_pubkey,
            &user.pool_account,
            &user.token_account,
            &test_pool.token_account.pubkey(),
            &test_pool.token_mint_pubkey,
            &test_pool.pool_mint.pubkey(),
            &wrong_user_authority.pubkey(),
            withdraw_amount,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &wrong_user_authority],
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
            InstructionError::Custom(spl_token::error::TokenError::OwnerMismatch as u32)
        )
    )
}

#[tokio::test]
async fn fail_with_invalid_withdraw_amount() {
    let (mut context, test_pool_market, test_pool, _pool_borrow_authority, user) = setup().await;

    test_pool
        .deposit(&mut context, &test_pool_market, &user, 100)
        .await
        .unwrap();

    context.warp_to_slot(WARP_SLOT + 5).unwrap();

    let withdraw_amount = u64::MAX;

    let tx = Transaction::new_signed_with_payer(
        &[instruction::withdraw_request(
            &everlend_general_pool::id(),
            &test_pool_market.keypair.pubkey(),
            &test_pool.pool_pubkey,
            &user.pool_account,
            &user.token_account,
            &test_pool.token_account.pubkey(),
            &test_pool.token_mint_pubkey,
            &test_pool.pool_mint.pubkey(),
            &user.pubkey(),
            withdraw_amount,
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
            InstructionError::Custom(EverlendError::MathOverflow as u32)
        )
    )
}
