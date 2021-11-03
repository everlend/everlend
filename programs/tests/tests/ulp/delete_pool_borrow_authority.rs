#![cfg(feature = "test-bpf")]

use crate::utils::*;
use everlend_ulp::state::AccountType;
use solana_program_test::*;

async fn setup() -> (ProgramTestContext, TestPoolMarket, TestPool) {
    let mut context = program_test().start_with_context().await;

    let test_pool_market = TestPoolMarket::new();
    test_pool_market.init(&mut context).await.unwrap();

    let test_pool = TestPool::new(&test_pool_market);
    test_pool
        .create(&mut context, &test_pool_market)
        .await
        .unwrap();

    (context, test_pool_market, test_pool)
}

#[tokio::test]
async fn success() {
    let (mut context, test_pool_market, test_pool) = setup().await;

    let test_pool_borrow_authority = TestPoolBorrowAuthority::new(&test_pool, None);
    test_pool_borrow_authority
        .create(&mut context, &test_pool_market, &test_pool, SHARE_ALLOWED)
        .await
        .unwrap();

    test_pool_borrow_authority
        .delete(&mut context, &test_pool_market, &test_pool)
        .await
        .unwrap();

    assert_eq!(
        context
            .banks_client
            .get_account(test_pool_borrow_authority.pool_borrow_authority_pubkey)
            .await
            .expect("account not found"),
        None,
    )
}

#[tokio::test]
async fn success_recreate() {
    let (mut context, test_pool_market, test_pool) = setup().await;

    let test_pool_borrow_authority = TestPoolBorrowAuthority::new(&test_pool, None);
    test_pool_borrow_authority
        .create(&mut context, &test_pool_market, &test_pool, SHARE_ALLOWED)
        .await
        .unwrap();

    test_pool_borrow_authority
        .delete(&mut context, &test_pool_market, &test_pool)
        .await
        .unwrap();

    context.warp_to_slot(3).unwrap();

    test_pool_borrow_authority
        .create(&mut context, &test_pool_market, &test_pool, SHARE_ALLOWED)
        .await
        .unwrap();

    let pool_borrow_authority = test_pool_borrow_authority.get_data(&mut context).await;

    assert_eq!(
        pool_borrow_authority.account_type,
        AccountType::PoolBorrowAuthority
    );
}
