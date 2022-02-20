#![cfg(feature = "test-bpf")]

use crate::utils::*;
use everlend_income_pools::state::AccountType;
use solana_program_test::*;

async fn setup() -> (ProgramTestContext, TestIncomePoolMarket) {
    let mut context = presetup().await.0;

    let general_pool_market = TestGeneralPoolMarket::new();
    general_pool_market.init(&mut context).await.unwrap();

    let test_income_pool_market = TestIncomePoolMarket::new();
    test_income_pool_market
        .init(&mut context, &general_pool_market)
        .await
        .unwrap();

    (context, test_income_pool_market)
}

#[tokio::test]
async fn success() {
    let (mut context, test_income_pool_market) = setup().await;

    let test_income_pool = TestIncomePool::new(&test_income_pool_market, None);
    test_income_pool
        .create(&mut context, &test_income_pool_market)
        .await
        .unwrap();

    let pool = test_income_pool.get_data(&mut context).await;

    assert_eq!(pool.account_type, AccountType::IncomePool);
}
