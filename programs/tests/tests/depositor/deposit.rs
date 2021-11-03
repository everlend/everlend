#![cfg(feature = "test-bpf")]

use crate::utils::*;
use solana_program_test::*;
use solana_sdk::signer::Signer;

async fn setup() -> (
    ProgramTestContext,
    TestPoolMarket,
    TestPool,
    TestPoolBorrowAuthority,
    LiquidityProvider,
) {
    let mut context = program_test().start_with_context().await;

    let test_pool_market = TestPoolMarket::new();
    test_pool_market.init(&mut context).await.unwrap();

    let test_pool = TestPool::new(&test_pool_market);
    test_pool
        .create(&mut context, &test_pool_market)
        .await
        .unwrap();

    let test_pool_borrow_authority = TestPoolBorrowAuthority::new(&test_pool, None);
    test_pool_borrow_authority
        .create(&mut context, &test_pool_market, &test_pool, SHARE_ALLOWED)
        .await
        .unwrap();

    let liquidity_provider = add_liquidity_provider(&mut context, &test_pool, 9999 * EXP)
        .await
        .unwrap();

    (
        context,
        test_pool_market,
        test_pool,
        test_pool_borrow_authority,
        liquidity_provider,
    )
}

#[tokio::test]
async fn success() {
    let (mut context, test_pool_market, test_pool, test_pool_borrow_authority, liquidity_provider) =
        setup().await;

    // test_pool
    //     .deposit(&mut context, &test_pool_market, &user, 100)
    //     .await
    //     .unwrap();

    // assert_eq!(
    //     get_token_balance(&mut context, &user.destination).await,
    //     100,
    // );
}
