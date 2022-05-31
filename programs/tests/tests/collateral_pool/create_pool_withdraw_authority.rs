#![cfg(feature = "test-bpf")]

use everlend_collateral_pool::state::AccountType;
use solana_program_test::*;
use solana_sdk::{signer::Signer};
use crate::utils::{
    presetup,
    TestPoolMarket,
    TestPool,
    TestPoolWithdrawAuthority,
};

async fn setup() -> (ProgramTestContext, TestPoolMarket, TestPool) {
    let mut context = presetup().await.0;

    let test_pool_market = TestPoolMarket::new();
    test_pool_market.init(&mut context).await.unwrap();

    let test_pool = TestPool::new(&test_pool_market, None);
    test_pool
        .create(&mut context, &test_pool_market)
        .await
        .unwrap();

    (context, test_pool_market, test_pool)
}

#[tokio::test]
async fn success() {
    let (mut context, test_pool_market, test_pool) = setup().await;

    let withdraw_authority_pubkey = context.payer.pubkey();
    let test_pool_withdraw_authority =
        TestPoolWithdrawAuthority::new(&test_pool, &withdraw_authority_pubkey);
    test_pool_withdraw_authority
        .create(
            &mut context,
            &test_pool_market,
            &test_pool,
            &withdraw_authority_pubkey,
        )
        .await
        .unwrap();

    let pool_withdraw_authority = test_pool_withdraw_authority.get_data(&mut context).await;

    assert_eq!(
        pool_withdraw_authority.account_type,
        AccountType::TestPoolWithdrawAuthority
    );
}
