#![cfg(feature = "test-bpf")]

use solana_program_test::*;

use everlend_ulp::state::AccountType;

use crate::utils::*;

#[tokio::test]
async fn success() {
    let mut context = presetup().await.0;

    let test_pool_market = TestUlpPoolMarket::new();
    test_pool_market.init(&mut context).await.unwrap();

    let pool_market = test_pool_market.get_data(&mut context).await;

    assert_eq!(pool_market.account_type, AccountType::PoolMarket);
}
