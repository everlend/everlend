#![cfg(feature = "test-bpf")]

use crate::utils::*;
use solana_program_test::*;
use solana_sdk::signer::Signer;

async fn setup() -> (
    ProgramTestContext,
    TestIncomePoolMarket,
    TestIncomePool,
    TokenHolder,
) {
    let mut context = presetup().await.0;

    let general_pool_market = TestGeneralPoolMarket::new();
    general_pool_market.init(&mut context).await.unwrap();

    let test_income_pool_market = TestIncomePoolMarket::new();
    test_income_pool_market
        .init(&mut context, &general_pool_market)
        .await
        .unwrap();

    let test_income_pool = TestIncomePool::new(&test_income_pool_market, None);
    test_income_pool
        .create(&mut context, &test_income_pool_market)
        .await
        .unwrap();

    let user = add_token_holder(
        &mut context,
        &test_income_pool.token_mint_pubkey,
        9999 * EXP,
    )
    .await
    .unwrap();

    (context, test_income_pool_market, test_income_pool, user)
}

#[tokio::test]
async fn success() {
    let (mut context, test_income_pool_market, test_income_pool, user) = setup().await;

    test_income_pool
        .deposit(&mut context, &test_income_pool_market, &user, 100)
        .await
        .unwrap();

    assert_eq!(
        get_token_balance(&mut context, &test_income_pool.token_account.pubkey()).await,
        100
    );
}
