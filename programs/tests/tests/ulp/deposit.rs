#![cfg(feature = "test-bpf")]

use crate::utils::*;
use solana_program_test::*;
use solana_sdk::signer::Signer;

async fn setup() -> (
    ProgramTestContext,
    TestPoolMarket,
    TestPool,
    LiquidityProvider,
) {
    let mut context = presetup().await.0;

    let test_pool_market = TestPoolMarket::new();
    test_pool_market.init(&mut context).await.unwrap();

    let test_pool = TestPool::new(&test_pool_market, None);
    test_pool
        .create(&mut context, &test_pool_market)
        .await
        .unwrap();

    let user = add_liquidity_provider(&mut context, &test_pool, 9999 * EXP)
        .await
        .unwrap();

    (context, test_pool_market, test_pool, user)
}

#[tokio::test]
async fn success() {
    let (mut context, test_pool_market, test_pool, user) = setup().await;

    test_pool
        .deposit(&mut context, &test_pool_market, &user, 100)
        .await
        .unwrap();

    assert_eq!(
        get_token_balance(&mut context, &user.pool_account).await,
        100,
    );
}

#[tokio::test]
async fn success_with_rate() {
    let (mut context, test_pool_market, test_pool, user) = setup().await;
    let a = (100 * EXP, 50 * EXP, 100 * EXP); // Deposit -> Raise -> Deposit

    // 0. Deposit to 100
    test_pool
        .deposit(&mut context, &test_pool_market, &user, a.0)
        .await
        .unwrap();

    // 1. Raise total incoming token
    mint_tokens(
        &mut context,
        &test_pool.token_mint_pubkey,
        &test_pool.token_account.pubkey(),
        a.1,
    )
    .await
    .unwrap();

    // Update slot for next deposit
    context.warp_to_slot(3).unwrap();

    // 2. More deposit with changed rate
    test_pool
        .deposit(&mut context, &test_pool_market, &user, a.2)
        .await
        .unwrap();

    // Around 166
    let destination_amount = a.0 + (a.2 as u128 * a.0 as u128 / (a.0 + a.1) as u128) as u64;

    assert_eq!(
        get_token_balance(&mut context, &user.pool_account).await,
        destination_amount
    );
}
