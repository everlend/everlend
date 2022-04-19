#![cfg(feature = "test-bpf")]

use solana_program_test::*;

use everlend_ulp::state::AccountType;

use crate::utils::*;

async fn setup() -> (ProgramTestContext, TestUlpPoolMarket) {
    let (context, .., test_pool_market) = presetup().await;

    (context, test_pool_market)
}

#[tokio::test]
async fn success() {
    let (mut context, test_pool_market) = setup().await;

    let test_pool = TestPool::new(&test_pool_market, None);
    test_pool
        .create(&mut context, &test_pool_market)
        .await
        .unwrap();

    let pool = test_pool.get_data(&mut context).await;

    assert_eq!(pool.account_type, AccountType::Pool);
}
