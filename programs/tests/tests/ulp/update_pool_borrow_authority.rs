#![cfg(feature = "test-bpf")]

use crate::utils::*;
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
        .update(&mut context, &test_pool_market, &test_pool, 2_000)
        .await
        .unwrap();

    assert_eq!(
        test_pool_borrow_authority
            .get_data(&mut context)
            .await
            .share_allowed,
        2_000
    )
}
