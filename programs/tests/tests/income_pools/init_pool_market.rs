#![cfg(feature = "test-bpf")]

use crate::utils::*;
use everlend_income_pools::state::AccountType;
use solana_program_test::*;

#[tokio::test]
async fn success() {
    let mut context = presetup().await.0;

    let general_pool_market = TestGeneralPoolMarket::new();
    general_pool_market.init(&mut context).await.unwrap();

    let test_income_pool_market = TestIncomePoolMarket::new();
    test_income_pool_market
        .init(&mut context, &general_pool_market)
        .await
        .unwrap();

    let pool_market = test_income_pool_market.get_data(&mut context).await;

    assert_eq!(pool_market.account_type, AccountType::IncomePoolMarket);
}
