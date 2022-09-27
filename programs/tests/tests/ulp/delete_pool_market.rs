#![cfg(feature = "test-bpf")]

use solana_program_test::*;
use crate::utils::{
    presetup,
    UlpMarket,
};

async fn setup() -> (ProgramTestContext, UlpMarket) {
    let mut context = presetup().await.context;

    let test_pool_market = UlpMarket::new();
    test_pool_market.init(&mut context).await.unwrap();

    (context, test_pool_market)
}

#[tokio::test]
async fn success() {
    let (mut context, test_pool_market) = setup().await;

    test_pool_market
        .delete(&mut context)
        .await
        .unwrap();
}