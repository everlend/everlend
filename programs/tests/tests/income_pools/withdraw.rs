#![cfg(feature = "test-bpf")]

use crate::utils::*;
use solana_program_test::*;
use solana_sdk::signer::Signer;

const TOKEN_AMOUNT: u64 = 123 * EXP;

async fn setup() -> (
    ProgramTestContext,
    TestIncomePoolMarket,
    TestIncomePool,
    TestGeneralPool,
) {
    let mut context = presetup().await.0;

    let general_pool_market = TestGeneralPoolMarket::new();
    general_pool_market.init(&mut context).await.unwrap();

    let test_income_pool_market = TestIncomePoolMarket::new();
    test_income_pool_market
        .init(&mut context, &general_pool_market)
        .await
        .unwrap();

    let test_general_pool = TestGeneralPool::new(&general_pool_market, None);
    test_general_pool
        .create(&mut context, &general_pool_market)
        .await
        .unwrap();

    let test_income_pool = TestIncomePool::new(&test_income_pool_market, None);
    test_income_pool
        .create(&mut context, &test_income_pool_market)
        .await
        .unwrap();

    mint_tokens(
        &mut context,
        &test_income_pool.token_mint_pubkey,
        &test_income_pool.token_account.pubkey(),
        TOKEN_AMOUNT,
    )
    .await
    .unwrap();

    (
        context,
        test_income_pool_market,
        test_income_pool,
        test_general_pool,
    )
}

#[tokio::test]
async fn success() {
    let (mut context, test_income_pool_market, test_income_pool, test_general_pool) = setup().await;

    assert_eq!(
        get_token_balance(&mut context, &test_income_pool.token_account.pubkey()).await,
        TOKEN_AMOUNT
    );

    test_income_pool
        .withdraw(&mut context, &test_income_pool_market, &test_general_pool)
        .await
        .unwrap();

    assert_eq!(
        get_token_balance(&mut context, &test_general_pool.token_account.pubkey()).await,
        TOKEN_AMOUNT
    );
}
