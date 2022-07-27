#![cfg(feature = "test-bpf")]

use everlend_ulp::state::AccountType;
use solana_program_test::*;
use crate::utils::{
    presetup,
    UlpMarket,
};

#[tokio::test]
async fn success() {
    let mut context = presetup().await.context;

    let test_pool_market = UlpMarket::new();
    test_pool_market.init(&mut context).await.unwrap();

    let pool_market = test_pool_market.get_data(&mut context).await;

    assert_eq!(pool_market.account_type, AccountType::PoolMarket);
}
