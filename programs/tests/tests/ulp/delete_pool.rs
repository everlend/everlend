#![cfg(feature = "test-bpf")]

use solana_program_test::*;
use crate::utils::{
    presetup,
    UlpMarket,
    UniversalLiquidityPool,
};

async fn setup() -> (ProgramTestContext, UlpMarket, UniversalLiquidityPool) {
    let mut context = presetup().await.context;

    let test_pool_market = UlpMarket::new();
    test_pool_market.init(&mut context).await.unwrap();

    let test_pool = UniversalLiquidityPool::new(&test_pool_market, None);
    test_pool
        .create(&mut context, &test_pool_market)
        .await
        .unwrap();

    (context, test_pool_market, test_pool)
}

#[tokio::test]
async fn success() {
    let (mut context, test_pool_market, test_pool) = setup().await;

    test_pool
        .delete_pool(&mut context, &test_pool_market)
        .await
        .unwrap();
}
