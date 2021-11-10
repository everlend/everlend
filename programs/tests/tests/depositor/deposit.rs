#![cfg(feature = "test-bpf")]

use crate::utils::*;
use solana_program_test::*;
use solana_sdk::{signature::Keypair, signer::Signer};

async fn setup() -> (
    ProgramTestContext,
    TestPoolMarket,
    TestPool,
    TestPoolBorrowAuthority,
    LiquidityProvider,
    TestDepositor,
) {
    let mut context = program_test().start_with_context().await;

    let rebalancer = Keypair::new();

    // 0. Prepare pool

    let test_pool_market = TestPoolMarket::new();
    test_pool_market.init(&mut context).await.unwrap();

    let test_pool = TestPool::new(&test_pool_market);
    test_pool
        .create(&mut context, &test_pool_market)
        .await
        .unwrap();

    let borrow_authority = Keypair::from_bytes(&rebalancer.to_bytes()).unwrap();
    let test_pool_borrow_authority =
        TestPoolBorrowAuthority::new(&test_pool, Some(borrow_authority));
    test_pool_borrow_authority
        .create(&mut context, &test_pool_market, &test_pool, SHARE_ALLOWED)
        .await
        .unwrap();

    let liquidity_provider = add_liquidity_provider(&mut context, &test_pool, 9999 * EXP)
        .await
        .unwrap();

    test_pool
        .deposit(
            &mut context,
            &test_pool_market,
            &liquidity_provider,
            100 * EXP,
        )
        .await
        .unwrap();

    // 1. Prepare depositor

    let test_depositor = TestDepositor::new(Some(rebalancer));
    test_depositor.init(&mut context).await.unwrap();

    test_depositor
        .create_transit(&mut context, &test_pool.token_mint.pubkey())
        .await
        .unwrap();

    (
        context,
        test_pool_market,
        test_pool,
        test_pool_borrow_authority,
        liquidity_provider,
        test_depositor,
    )
}

#[tokio::test]
async fn success() {
    let (
        mut context,
        test_pool_market,
        test_pool,
        test_pool_borrow_authority,
        liquidity_provider,
        test_depositor,
    ) = setup().await;

    test_depositor
        .deposit(
            &mut context,
            &test_pool_market,
            &test_pool,
            &test_pool_borrow_authority,
            100,
        )
        .await
        .unwrap();

    // assert_eq!(
    //     get_token_balance(&mut context, &user.destination).await,
    //     100,
    // );
}
