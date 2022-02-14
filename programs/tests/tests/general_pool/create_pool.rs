#![cfg(feature = "test-bpf")]

use crate::utils::*;
use everlend_general_pool::state::AccountType;
use solana_program_test::*;

async fn setup() -> (ProgramTestContext, TestGeneralPoolMarket) {
    let mut context = presetup().await.0;

    let test_pool_market = TestGeneralPoolMarket::new();
    test_pool_market.init(&mut context).await.unwrap();

    (context, test_pool_market)
}

#[tokio::test]
async fn success() {
    let (mut context, test_pool_market) = setup().await;

    let test_pool = TestGeneralPool::new(&test_pool_market, None);
    test_pool
        .create(&mut context, &test_pool_market)
        .await
        .unwrap();

    let pool = test_pool.get_data(&mut context).await;

    assert_eq!(pool.account_type, AccountType::Pool);

    let requests =  test_pool.get_withdraw_requests(& mut context, &everlend_general_pool::id()).await;
    assert_eq!(
        requests.account_type,
        AccountType::WithdrawRequests
    );

    assert_eq!(
        requests.pool,
        test_pool.pool_pubkey,
    );
}
