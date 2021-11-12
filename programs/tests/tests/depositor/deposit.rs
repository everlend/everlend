#![cfg(feature = "test-bpf")]

use crate::utils::*;
use solana_program_test::*;
use solana_sdk::{signature::Keypair, signer::Signer};

async fn setup() -> (
    ProgramTestContext,
    TestPoolMarket,
    TestPool,
    TestPoolBorrowAuthority,
    TestPoolMarket,
    TestPool,
    LiquidityProvider,
    TestDepositor,
) {
    let mut context = program_test().start_with_context().await;

    let rebalancer = Keypair::new();

    // 0. Prepare pool

    let general_pool_market = TestPoolMarket::new();
    general_pool_market.init(&mut context).await.unwrap();

    let general_pool = TestPool::new(&general_pool_market);
    general_pool
        .create(&mut context, &general_pool_market)
        .await
        .unwrap();

    let borrow_authority = Keypair::from_bytes(&rebalancer.to_bytes()).unwrap();
    let general_pool_borrow_authority =
        TestPoolBorrowAuthority::new(&general_pool, Some(borrow_authority));
    general_pool_borrow_authority
        .create(
            &mut context,
            &general_pool_market,
            &general_pool,
            SHARE_ALLOWED,
        )
        .await
        .unwrap();

    let liquidity_provider = add_liquidity_provider(&mut context, &general_pool, 9999 * EXP)
        .await
        .unwrap();

    general_pool
        .deposit(
            &mut context,
            &general_pool_market,
            &liquidity_provider,
            100 * EXP,
        )
        .await
        .unwrap();

    // 1. Prepare money market pool

    let mm_pool_market = TestPoolMarket::new();
    mm_pool_market.init(&mut context).await.unwrap();

    let mm_pool = TestPool::new(&mm_pool_market);
    mm_pool.create(&mut context, &mm_pool_market).await.unwrap();

    // 2. Prepare depositor

    let test_depositor = TestDepositor::new(Some(rebalancer));
    test_depositor.init(&mut context).await.unwrap();

    // 2.1 Create transit account for liquidity token
    test_depositor
        .create_transit(&mut context, &general_pool.token_mint.pubkey())
        .await
        .unwrap();

    // 2.2 Create transit account for collateral token
    test_depositor
        .create_transit(&mut context, &mm_pool.token_mint.pubkey())
        .await
        .unwrap();

    (
        context,
        general_pool_market,
        general_pool,
        general_pool_borrow_authority,
        mm_pool_market,
        mm_pool,
        liquidity_provider,
        test_depositor,
    )
}

#[tokio::test]
async fn success() {
    let (
        mut context,
        general_pool_market,
        general_pool,
        general_pool_borrow_authority,
        mm_pool_market,
        mm_pool,
        liquidity_provider,
        test_depositor,
    ) = setup().await;

    test_depositor
        .deposit(
            &mut context,
            &general_pool_market,
            &general_pool,
            &general_pool_borrow_authority,
            &mm_pool_market,
            &mm_pool,
            100,
        )
        .await
        .unwrap();

    // assert_eq!(
    //     get_token_balance(&mut context, &user.destination).await,
    //     100,
    // );
}
