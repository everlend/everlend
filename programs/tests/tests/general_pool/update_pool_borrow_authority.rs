use solana_program::instruction::InstructionError;
use solana_program::pubkey::Pubkey;
use solana_program_test::*;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use solana_sdk::transaction::{Transaction, TransactionError};

use everlend_general_pool::instruction;
use everlend_utils::EverlendError;

use crate::utils::*;

async fn setup() -> (ProgramTestContext, TestRegistry, TestGeneralPoolMarket, TestGeneralPool) {
    let mut env = presetup().await;

    let test_pool_market = TestGeneralPoolMarket::new();
    test_pool_market.init(&mut env.context, &env.registry.keypair.pubkey()).await.unwrap();

    let test_pool = TestGeneralPool::new(&test_pool_market, None);
    test_pool
        .create(&mut env.context, &test_pool_market)
        .await
        .unwrap();

    (env.context, env.registry, test_pool_market, test_pool)
}

#[tokio::test]
async fn success() {
    let (mut context, _, test_pool_market, test_pool) = setup().await;

    let test_pool_borrow_authority =
        TestGeneralPoolBorrowAuthority::new(&test_pool, context.payer.pubkey());
    test_pool_borrow_authority
        .create(
            &mut context,
            &test_pool_market,
            &test_pool,
            GENERAL_POOL_SHARE_ALLOWED,
        )
        .await
        .unwrap();

    let new_share_allowed = 2_000;

    test_pool_borrow_authority
        .update(
            &mut context,
            &test_pool_market,
            &test_pool,
            new_share_allowed,
        )
        .await
        .unwrap();

    assert_eq!(
        test_pool_borrow_authority
            .get_data(&mut context)
            .await
            .share_allowed,
        new_share_allowed
    )
}

#[tokio::test]
async fn fail_update_with_fake_pool_market() {
    let (mut context, registry, test_pool_market, test_pool) = setup().await;

    let test_pool_borrow_authority =
        TestGeneralPoolBorrowAuthority::new(&test_pool, context.payer.pubkey());

    test_pool_borrow_authority
        .create(
            &mut context,
            &test_pool_market,
            &test_pool,
            GENERAL_POOL_SHARE_ALLOWED,
        )
        .await
        .unwrap();

    let fake_pool_market = TestGeneralPoolMarket::new();
    fake_pool_market.init(&mut context, &registry.keypair.pubkey()).await.unwrap();

    context.warp_to_slot(3).unwrap();

    let new_share_allowed = 2_000;

    let tx = Transaction::new_signed_with_payer(
        &[instruction::update_pool_borrow_authority(
            &everlend_general_pool::id(),
            &fake_pool_market.keypair.pubkey(),
            &test_pool.pool_pubkey,
            &test_pool_borrow_authority.borrow_authority,
            &fake_pool_market.manager.pubkey(),
            new_share_allowed,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &fake_pool_market.manager],
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
async fn fail_update_with_invalid_pool_market() {
    let (mut context, _, test_pool_market, test_pool) = setup().await;

    let test_pool_borrow_authority =
        TestGeneralPoolBorrowAuthority::new(&test_pool, context.payer.pubkey());

    test_pool_borrow_authority
        .create(
            &mut context,
            &test_pool_market,
            &test_pool,
            GENERAL_POOL_SHARE_ALLOWED,
        )
        .await
        .unwrap();

    context.warp_to_slot(3).unwrap();

    let new_share_allowed = 2_000;

    let tx = Transaction::new_signed_with_payer(
        &[instruction::update_pool_borrow_authority(
            &everlend_general_pool::id(),
            &Pubkey::new_unique(),
            &test_pool.pool_pubkey,
            &test_pool_borrow_authority.borrow_authority,
            &test_pool_market.manager.pubkey(),
            new_share_allowed,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &test_pool_market.manager],
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
async fn fail_update_with_invalid_pool() {
    let (mut context, _, test_pool_market, test_pool) = setup().await;

    let test_pool_borrow_authority =
        TestGeneralPoolBorrowAuthority::new(&test_pool, context.payer.pubkey());

    test_pool_borrow_authority
        .create(
            &mut context,
            &test_pool_market,
            &test_pool,
            GENERAL_POOL_SHARE_ALLOWED,
        )
        .await
        .unwrap();

    context.warp_to_slot(3).unwrap();

    let new_share_allowed = 2_000;

    let tx = Transaction::new_signed_with_payer(
        &[instruction::update_pool_borrow_authority(
            &everlend_general_pool::id(),
            &test_pool_market.keypair.pubkey(),
            &Pubkey::new_unique(),
            &test_pool_borrow_authority.borrow_authority,
            &test_pool_market.manager.pubkey(),
            new_share_allowed,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &test_pool_market.manager],
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
async fn fail_update_with_invalid_borrow_authority() {
    let (mut context, _, test_pool_market, test_pool) = setup().await;

    let test_pool_borrow_authority =
        TestGeneralPoolBorrowAuthority::new(&test_pool, context.payer.pubkey());

    test_pool_borrow_authority
        .create(
            &mut context,
            &test_pool_market,
            &test_pool,
            GENERAL_POOL_SHARE_ALLOWED,
        )
        .await
        .unwrap();

    context.warp_to_slot(3).unwrap();

    let new_share_allowed = 2_000;

    let tx = Transaction::new_signed_with_payer(
        &[instruction::update_pool_borrow_authority(
            &everlend_general_pool::id(),
            &test_pool_market.keypair.pubkey(),
            &test_pool.pool_pubkey,
            &Pubkey::new_unique(),
            &test_pool_market.manager.pubkey(),
            new_share_allowed,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &test_pool_market.manager],
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
async fn fail_update_with_wrong_manager() {
    let (mut context, _, test_pool_market, test_pool) = setup().await;

    let test_pool_borrow_authority =
        TestGeneralPoolBorrowAuthority::new(&test_pool, context.payer.pubkey());

    test_pool_borrow_authority
        .create(
            &mut context,
            &test_pool_market,
            &test_pool,
            GENERAL_POOL_SHARE_ALLOWED,
        )
        .await
        .unwrap();

    context.warp_to_slot(3).unwrap();

    let new_share_allowed = 2_000;

    let wrong_manager = Keypair::new();

    let tx = Transaction::new_signed_with_payer(
        &[instruction::update_pool_borrow_authority(
            &everlend_general_pool::id(),
            &test_pool_market.keypair.pubkey(),
            &test_pool.pool_pubkey,
            &test_pool_borrow_authority.borrow_authority,
            &wrong_manager.pubkey(),
            new_share_allowed,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &wrong_manager],
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
