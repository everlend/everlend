#![cfg(feature = "test-bpf")]

use solana_program::instruction::InstructionError;
use solana_program::pubkey::Pubkey;
use solana_program_test::*;
use solana_sdk::signer::Signer;
use solana_sdk::transaction::{Transaction, TransactionError};

use everlend_general_pool::state::WITHDRAW_DELAY;
use everlend_general_pool::{find_transit_program_address, instruction};
use everlend_utils::EverlendError;

use crate::utils::*;

const INITIAL_USER_BALANCE: u64 = 5000000;

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

    // Fill user account by native token
    transfer(&mut context, &user.owner.pubkey(), INITIAL_USER_BALANCE)
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

    context.warp_to_slot(3).unwrap();

    test_pool
        .withdraw_request(&mut context, &test_pool_market, &user, 45)
        .await
        .unwrap();

    context.warp_to_slot(3 + WITHDRAW_DELAY).unwrap();

    let (transit_account, _) = find_transit_program_address(
        &everlend_general_pool::id(),
        &test_pool_market.keypair.pubkey(),
        &test_pool.pool_mint.pubkey(),
    );
    assert_eq!(get_token_balance(&mut context, &transit_account).await, 45);

    test_pool
        .withdraw(&mut context, &test_pool_market, &user)
        .await
        .unwrap();

    assert_eq!(
        get_token_balance(&mut context, &user.pool_account).await,
        55
    );
    assert_eq!(
        get_token_balance(&mut context, &user.token_account).await,
        46
    );
    assert_eq!(
        get_token_balance(&mut context, &test_pool.token_account.pubkey()).await,
        55
    );
    assert_eq!(get_token_balance(&mut context, &transit_account).await, 0);

    let user_account = get_account(&mut context, &user.owner.pubkey()).await;
    assert_eq!(user_account.lamports, INITIAL_USER_BALANCE);
}

#[tokio::test]
async fn fail_with_invalid_ticket() {
    let (mut context, test_pool_market, test_pool, _pool_borrow_authority, user) = setup().await;

    test_pool
        .deposit(&mut context, &test_pool_market, &user, 100)
        .await
        .unwrap();
    context.warp_to_slot(3).unwrap();

    test_pool
        .withdraw_request(&mut context, &test_pool_market, &user, 45)
        .await
        .unwrap();
    context.warp_to_slot(3 + WITHDRAW_DELAY - 1).unwrap();

    assert_eq!(
        test_pool
            .withdraw(&mut context, &test_pool_market, &user)
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(EverlendError::WithdrawRequestsInvalidTicket as u32)
        )
    )
}

#[tokio::test]
async fn fail_with_invalid_pool_market() {
    let (mut context, test_pool_market, test_pool, _pool_borrow_authority, user) = setup().await;

    test_pool
        .deposit(&mut context, &test_pool_market, &user, 100)
        .await
        .unwrap();

    context.warp_to_slot(3).unwrap();

    test_pool
        .withdraw_request(&mut context, &test_pool_market, &user, 45)
        .await
        .unwrap();

    context.warp_to_slot(3 + WITHDRAW_DELAY).unwrap();

    let (transit_account, _) = find_transit_program_address(
        &everlend_general_pool::id(),
        &test_pool_market.keypair.pubkey(),
        &test_pool.pool_mint.pubkey(),
    );
    assert_eq!(get_token_balance(&mut context, &transit_account).await, 45);

    let tx = Transaction::new_signed_with_payer(
        &[instruction::withdraw(
            &everlend_general_pool::id(),
            &Pubkey::new_unique(),
            &test_pool.pool_pubkey,
            &user.token_account,
            &test_pool.token_account.pubkey(),
            &test_pool.token_mint_pubkey,
            &test_pool.pool_mint.pubkey(),
            &user.owner.pubkey(),
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
    )
}

#[tokio::test]
async fn fail_with_invalid_pool() {
    let (mut context, test_pool_market, test_pool, _pool_borrow_authority, user) = setup().await;

    test_pool
        .deposit(&mut context, &test_pool_market, &user, 100)
        .await
        .unwrap();

    context.warp_to_slot(3).unwrap();

    test_pool
        .withdraw_request(&mut context, &test_pool_market, &user, 45)
        .await
        .unwrap();

    context.warp_to_slot(3 + WITHDRAW_DELAY).unwrap();

    let (transit_account, _) = find_transit_program_address(
        &everlend_general_pool::id(),
        &test_pool_market.keypair.pubkey(),
        &test_pool.pool_mint.pubkey(),
    );
    assert_eq!(get_token_balance(&mut context, &transit_account).await, 45);

    let tx = Transaction::new_signed_with_payer(
        &[instruction::withdraw(
            &everlend_general_pool::id(),
            &test_pool_market.keypair.pubkey(),
            &Pubkey::new_unique(),
            &user.token_account,
            &test_pool.token_account.pubkey(),
            &test_pool.token_mint_pubkey,
            &test_pool.pool_mint.pubkey(),
            &user.owner.pubkey(),
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
    )
}

#[tokio::test]
async fn fail_with_invalid_destination() {
    let (mut context, test_pool_market, test_pool, _pool_borrow_authority, user) = setup().await;

    test_pool
        .deposit(&mut context, &test_pool_market, &user, 100)
        .await
        .unwrap();

    context.warp_to_slot(3).unwrap();

    test_pool
        .withdraw_request(&mut context, &test_pool_market, &user, 45)
        .await
        .unwrap();

    context.warp_to_slot(3 + WITHDRAW_DELAY).unwrap();

    let (transit_account, _) = find_transit_program_address(
        &everlend_general_pool::id(),
        &test_pool_market.keypair.pubkey(),
        &test_pool.pool_mint.pubkey(),
    );
    assert_eq!(get_token_balance(&mut context, &transit_account).await, 45);

    let tx = Transaction::new_signed_with_payer(
        &[instruction::withdraw(
            &everlend_general_pool::id(),
            &test_pool_market.keypair.pubkey(),
            &test_pool.pool_pubkey,
            &Pubkey::new_unique(),
            &test_pool.token_account.pubkey(),
            &test_pool.token_mint_pubkey,
            &test_pool.pool_mint.pubkey(),
            &user.owner.pubkey(),
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
    )
}

#[tokio::test]
async fn fail_with_invalid_token_account() {
    let (mut context, test_pool_market, test_pool, _pool_borrow_authority, user) = setup().await;

    test_pool
        .deposit(&mut context, &test_pool_market, &user, 100)
        .await
        .unwrap();

    context.warp_to_slot(3).unwrap();

    test_pool
        .withdraw_request(&mut context, &test_pool_market, &user, 45)
        .await
        .unwrap();

    context.warp_to_slot(3 + WITHDRAW_DELAY).unwrap();

    let (transit_account, _) = find_transit_program_address(
        &everlend_general_pool::id(),
        &test_pool_market.keypair.pubkey(),
        &test_pool.pool_mint.pubkey(),
    );
    assert_eq!(get_token_balance(&mut context, &transit_account).await, 45);

    let tx = Transaction::new_signed_with_payer(
        &[instruction::withdraw(
            &everlend_general_pool::id(),
            &test_pool_market.keypair.pubkey(),
            &test_pool.pool_pubkey,
            &user.token_account,
            &Pubkey::new_unique(),
            &test_pool.token_mint_pubkey,
            &test_pool.pool_mint.pubkey(),
            &user.owner.pubkey(),
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
    )
}

#[tokio::test]
async fn fail_with_invalid_token_mint() {
    let (mut context, test_pool_market, test_pool, _pool_borrow_authority, user) = setup().await;

    test_pool
        .deposit(&mut context, &test_pool_market, &user, 100)
        .await
        .unwrap();

    context.warp_to_slot(3).unwrap();

    test_pool
        .withdraw_request(&mut context, &test_pool_market, &user, 45)
        .await
        .unwrap();

    context.warp_to_slot(3 + WITHDRAW_DELAY).unwrap();

    let (transit_account, _) = find_transit_program_address(
        &everlend_general_pool::id(),
        &test_pool_market.keypair.pubkey(),
        &test_pool.pool_mint.pubkey(),
    );
    assert_eq!(get_token_balance(&mut context, &transit_account).await, 45);

    let tx = Transaction::new_signed_with_payer(
        &[instruction::withdraw(
            &everlend_general_pool::id(),
            &test_pool_market.keypair.pubkey(),
            &test_pool.pool_pubkey,
            &user.token_account,
            &test_pool.token_account.pubkey(),
            &Pubkey::new_unique(),
            &test_pool.pool_mint.pubkey(),
            &user.owner.pubkey(),
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
    )
}

#[tokio::test]
async fn fail_with_invalid_pool_mint() {
    let (mut context, test_pool_market, test_pool, _pool_borrow_authority, user) = setup().await;

    test_pool
        .deposit(&mut context, &test_pool_market, &user, 100)
        .await
        .unwrap();

    context.warp_to_slot(3).unwrap();

    test_pool
        .withdraw_request(&mut context, &test_pool_market, &user, 45)
        .await
        .unwrap();

    context.warp_to_slot(3 + WITHDRAW_DELAY).unwrap();

    let (transit_account, _) = find_transit_program_address(
        &everlend_general_pool::id(),
        &test_pool_market.keypair.pubkey(),
        &test_pool.pool_mint.pubkey(),
    );
    assert_eq!(get_token_balance(&mut context, &transit_account).await, 45);

    let tx = Transaction::new_signed_with_payer(
        &[instruction::withdraw(
            &everlend_general_pool::id(),
            &test_pool_market.keypair.pubkey(),
            &test_pool.pool_pubkey,
            &user.token_account,
            &test_pool.token_account.pubkey(),
            &test_pool.token_mint_pubkey,
            &Pubkey::new_unique(),
            &user.owner.pubkey(),
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
        TransactionError::InstructionError(0, InstructionError::InvalidAccountData)
    )
}

#[tokio::test]
async fn success_with_random_tx_signer() {
    let (mut context, test_pool_market, test_pool, _pool_borrow_authority, user) = setup().await;

    test_pool
        .deposit(&mut context, &test_pool_market, &user, 100)
        .await
        .unwrap();

    context.warp_to_slot(3).unwrap();

    test_pool
        .withdraw_request(&mut context, &test_pool_market, &user, 45)
        .await
        .unwrap();

    let random_tx_signer = TestGeneralPoolMarket::new();
    random_tx_signer.init(&mut context).await.unwrap();

    context.warp_to_slot(3 + WITHDRAW_DELAY).unwrap();

    let (transit_account, _) = find_transit_program_address(
        &everlend_general_pool::id(),
        &test_pool_market.keypair.pubkey(),
        &test_pool.pool_mint.pubkey(),
    );
    assert_eq!(get_token_balance(&mut context, &transit_account).await, 45);

    let tx = Transaction::new_signed_with_payer(
        &[instruction::withdraw(
            &everlend_general_pool::id(),
            &test_pool_market.keypair.pubkey(),
            &test_pool.pool_pubkey,
            &user.token_account,
            &test_pool.token_account.pubkey(),
            &test_pool.token_mint_pubkey,
            &test_pool.pool_mint.pubkey(),
            &user.owner.pubkey(),
        )],
        Some(&random_tx_signer.manager.pubkey()),
        &[&random_tx_signer.manager],
        context.last_blockhash,
    );

    context.banks_client.process_transaction(tx).await.unwrap()
}
