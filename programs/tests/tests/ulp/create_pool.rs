#![cfg(feature = "test-bpf")]

use crate::utils::*;
use everlend_ulp::state::AccountType;
use solana_program_test::*;

async fn setup() -> (ProgramTestContext, UlpMarket) {
    let mut context = presetup().await.0;

    let test_pool_market = UlpMarket::new();
    test_pool_market.init(&mut context).await.unwrap();

    (context, test_pool_market)
}

#[tokio::test]
async fn success() {
    let (mut context, test_pool_market) = setup().await;

    let test_pool = UniversalLiquidityPool::new(&test_pool_market, None);
    test_pool
        .create(&mut context, &test_pool_market)
        .await
        .unwrap();

    let pool = test_pool.get_data(&mut context).await;

    assert_eq!(pool.account_type, AccountType::Pool);
}
