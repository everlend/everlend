#![cfg(feature = "test-bpf")]

use crate::utils::*;
use everlend_depositor::state::AccountType;
use solana_program_test::*;

#[tokio::test]
async fn success() {
    let (mut context, _, _, registry) = presetup().await;

    let test_liquidity_oracle = TestLiquidityOracle::new();
    test_liquidity_oracle.init(&mut context).await.unwrap();

    let general_pool_market = TestGeneralPoolMarket::new();
    general_pool_market.init(&mut context).await.unwrap();

    let income_pool_market = TestIncomePoolMarket::new();
    income_pool_market
        .init(&mut context, &general_pool_market)
        .await
        .unwrap();

    let test_depositor = TestDepositor::new();
    test_depositor
        .init(
            &mut context,
            &registry,
            &general_pool_market,
            &income_pool_market,
            &test_liquidity_oracle,
        )
        .await
        .unwrap();

    let depositor = test_depositor.get_data(&mut context).await;

    assert_eq!(depositor.account_type, AccountType::Depositor);
}
