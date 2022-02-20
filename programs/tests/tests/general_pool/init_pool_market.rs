#![cfg(feature = "test-bpf")]

use crate::utils::*;
use everlend_general_pool::state::AccountType;
use solana_program_test::*;
use crate::utils::general_pool_market::TestGeneralPoolMarket;

#[tokio::test]
async fn success() {
    let mut context = presetup().await.0;

    let test_pool_market = TestGeneralPoolMarket::new();
    test_pool_market.init(&mut context).await.unwrap();

    let pool_market = test_pool_market.get_data(&mut context).await;

    assert_eq!(pool_market.account_type, AccountType::PoolMarket);
}
