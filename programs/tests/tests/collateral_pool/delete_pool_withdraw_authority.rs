#![cfg(feature = "test-bpf")]

use everlend_collateral_pool::state::AccountType;
use everlend_utils::EverlendError;
use solana_program::instruction::InstructionError;
use solana_program_test::*;
use solana_sdk::{signer::Signer, transaction::TransactionError};

use crate::utils::{
    presetup,
    TestPoolMarket,
    TestPool,
    TestPoolBorrowAuthority,
};

async fn setup() -> (ProgramTestContext, TestPoolMarket, TestPool) {
    let mut context = presetup().await.context;

    let test_pool_market = TestPoolMarket::new();
    test_pool_market.init(&mut context).await.unwrap();

    let test_pool = TestPool::new(&test_pool_market, None);
    test_pool
        .create(&mut context, &test_pool_market)
        .await
        .unwrap();

    (context, test_pool_market, test_pool)
}

#[tokio::test]
async fn success() {
    let (mut context, test_pool_market, test_pool) = setup().await;

    let test_pool_withdraw_authority =
        TestPoolWithdrawAuthority::new(&test_pool, context.payer.pubkey());
    test_pool_withdraw_authority
        .create(
            &mut context,
            &test_pool_market,
            &test_pool,
        )
        .await
        .unwrap();

    test_pool_withdraw_authority
        .delete(&mut context, &test_pool_market, &test_pool)
        .await
        .unwrap();

    assert_eq!(
        context
            .banks_client
            .get_account(test_pool_withdraw_authority.pool_withdraw_authority_pubkey)
            .await
            .expect("account not found"),
        None,
    )
}

#[tokio::test]
async fn success_recreate() {
    let (mut context, test_pool_market, test_pool) = setup().await;

    let test_pool_withdraw_authority =
        TestPoolWithdrawAuthority::new(&test_pool, context.payer.pubkey());
    test_pool_withdraw_authority
        .create(
            &mut context,
            &test_pool_market,
            &test_pool,
        )
        .await
        .unwrap();

    test_pool_withdraw_authority
        .delete(&mut context, &test_pool_market, &test_pool)
        .await
        .unwrap();

    context.warp_to_slot(3).unwrap();

    test_pool_withdraw_authority
        .create(
            &mut context,
            &test_pool_market,
            &test_pool,
        )
        .await
        .unwrap();

    let test_pool_withdraw_authority = test_pool_borrow_authority.get_data(&mut context).await;

    assert_eq!(
        pool_withdraw_authority.account_type,
        AccountType::PoolWithdrawAuthority
    );
}

#[tokio::test]
async fn fail_delete_pool_borrow_authority() {
    let (mut context, test_pool_market, test_pool) = setup().await;

    let test_pool_borrow_authority =
        TestPoolBorrowAuthority::new(&test_pool, context.payer.pubkey());

    assert_eq!(
        test_pool_borrow_authority
            .delete(&mut context, &test_pool_market, &test_pool)
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(EverlendError::InvalidAccountOwner as u32)
        )
    );
}
