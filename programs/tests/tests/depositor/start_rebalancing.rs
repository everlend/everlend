#![cfg(feature = "test-bpf")]

use crate::utils::*;
use everlend_liquidity_oracle::state::{DistributionArray, LiquidityDistribution};
use solana_program_test::*;
use solana_sdk::signer::Signer;

async fn setup() -> (
    ProgramTestContext,
    TestDepositor,
    TestPoolMarket,
    TestPool,
    TestLiquidityOracle,
) {
    let mut context = presetup().await.0;

    let payer_pubkey = context.payer.pubkey();

    // 1. Prepare general pool

    let general_pool_market = TestPoolMarket::new();
    general_pool_market.init(&mut context).await.unwrap();

    let general_pool = TestPool::new(&general_pool_market, None);
    general_pool
        .create(&mut context, &general_pool_market)
        .await
        .unwrap();

    // 1.1 Add liquidity to general pool

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

    // 2. Prepare income pool
    let income_pool_market = TestIncomePoolMarket::new();
    income_pool_market
        .init(&mut context, &general_pool_market)
        .await
        .unwrap();

    // 3. Prepare liquidity oracle

    let test_liquidity_oracle = TestLiquidityOracle::new();
    test_liquidity_oracle.init(&mut context).await.unwrap();

    let mut distribution = DistributionArray::default();
    distribution[0] = LiquidityDistribution {
        money_market: spl_token_lending::id(),
        percent: 500_000_000u64, // 50%
    };

    let test_token_distribution =
        TestTokenDistribution::new(general_pool.token_mint_pubkey, distribution);

    test_token_distribution
        .init(&mut context, &test_liquidity_oracle, payer_pubkey)
        .await
        .unwrap();

    test_token_distribution
        .update(
            &mut context,
            &test_liquidity_oracle,
            payer_pubkey,
            distribution,
        )
        .await
        .unwrap();

    // 4. Prepare depositor

    let test_depositor = TestDepositor::new();
    test_depositor
        .init(
            &mut context,
            &general_pool_market,
            &income_pool_market,
            &test_liquidity_oracle,
        )
        .await
        .unwrap();

    (
        context,
        test_depositor,
        general_pool_market,
        general_pool,
        test_liquidity_oracle,
    )
}

#[tokio::test]
async fn success() {
    let (mut context, test_depositor, general_pool_market, general_pool, test_liquidity_oracle) =
        setup().await;

    test_depositor
        .start_rebalancing(
            &mut context,
            &general_pool_market,
            &general_pool,
            &test_liquidity_oracle,
        )
        .await
        .unwrap();
}
