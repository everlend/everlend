#![cfg(feature = "test-bpf")]

use everlend_ulp::state::AccountType;
use solana_program_test::*;
use solana_sdk::signer::Signer;
use crate::utils::{
    presetup,
    UlpMarket,
    UniversalLiquidityPool,
    UniversalLiquidityPoolBorrowAuthority,
};

pub const ULP_SHARE_ALLOWED: u16 = 10_000;

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

    let test_pool_borrow_authority =
        UniversalLiquidityPoolBorrowAuthority::new(&test_pool, context.payer.pubkey());
    test_pool_borrow_authority
        .create(
            &mut context,
            &test_pool_market,
            &test_pool,
            ULP_SHARE_ALLOWED,
        )
        .await
        .unwrap();

    let pool_borrow_authority = test_pool_borrow_authority.get_data(&mut context).await;

    assert_eq!(
        pool_borrow_authority.account_type,
        AccountType::PoolBorrowAuthority
    );
}
