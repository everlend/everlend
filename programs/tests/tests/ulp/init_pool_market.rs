#![cfg(feature = "test-bpf")]

use crate::utils::*;
use everlend_ulp::state::AccountType;
use solana_program_test::*;

#[tokio::test]
async fn success() {
    let mut context = program_test().start_with_context().await;

    let test_pool_market = TestPoolMarket::new();
    test_pool_market.init(&mut context).await.unwrap();

    let pool_market = test_pool_market.get_data(&mut context).await;

    assert_eq!(pool_market.account_type, AccountType::PoolMarket);
}
