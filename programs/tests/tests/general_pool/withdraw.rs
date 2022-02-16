#![cfg(feature = "test-bpf")]

use crate::utils::*;
use everlend_general_pool::find_transit_program_address;
use solana_program_test::*;
use solana_sdk::signer::Signer;

async fn setup() -> (
    ProgramTestContext,
    TestGeneralPoolMarket,
    TestGeneralPool,
    TestGeneralPoolBorrowAuthority,
    LiquidityProvider,
) {
    let mut context = presetup().await.0;

    let test_pool_market = TestGeneralPoolMarket::new();
    test_pool_market.init(&mut context).await.unwrap();

    let test_pool = TestGeneralPool::new(&test_pool_market, None);
    test_pool
        .create(&mut context, &test_pool_market)
        .await
        .unwrap();

    let test_pool_borrow_authority =
        TestGeneralPoolBorrowAuthority::new(&test_pool, context.payer.pubkey());
    test_pool_borrow_authority
        .create(
            &mut context,
            &test_pool_market,
            &test_pool,
            ULP_SHARE_ALLOWED,
        )
        .await
        .unwrap();

    let user = add_liquidity_provider(
        &mut context,
        &test_pool.token_mint_pubkey,
        &test_pool.pool_mint.pubkey(),
        101,
    )
    .await
    .unwrap();

    // Fill user account by native token
    transfer(&mut context, &user.owner.pubkey(), 10108000)
        .await
        .unwrap();

    test_pool
        .deposit(&mut context, &test_pool_market, &user, 100)
        .await
        .unwrap();

    (
        context,
        test_pool_market,
        test_pool,
        test_pool_borrow_authority,
        user,
    )
}

#[tokio::test]
async fn success() {
    let (mut context, test_pool_market, test_pool, _pool_borrow_authority, user) = setup().await;

    test_pool
        .deposit(&mut context, &test_pool_market, &user, 100)
        .await
        .unwrap();

    test_pool
        .withdraw_request(&mut context, &test_pool_market, &user, 45)
        .await
        .unwrap();

    let (transit_account, _) = find_transit_program_address(
        &everlend_general_pool::id(),
        &test_pool_market.keypair.pubkey(),
        &test_pool.pool_mint.pubkey(),
    );
    assert_eq!(get_token_balance(&mut context, &transit_account).await, 45);

    test_pool
        .withdraw(&mut context, &test_pool_market, &user, 0)
        .await
        .unwrap();

    assert_eq!(
        get_token_balance(&mut context, &user.pool_account).await,
        55
    );
    assert_eq!(
        get_token_balance(&mut context, &user.token_account).await,
        46
    );
    assert_eq!(
        get_token_balance(&mut context, &test_pool.token_account.pubkey()).await,
        55
    );

    assert_eq!(get_token_balance(&mut context, &transit_account).await, 0);
}

#[tokio::test]
async fn success_with_index() {
    let (mut context, test_pool_market, test_pool, _pool_borrow_authority, user) = setup().await;

    test_pool
        .deposit(&mut context, &test_pool_market, &user, 100)
        .await
        .unwrap();

    test_pool
        .withdraw_request(&mut context, &test_pool_market, &user, 50)
        .await
        .unwrap();

    test_pool
        .withdraw_request(&mut context, &test_pool_market, &user, 30)
        .await
        .unwrap();

    test_pool
        .withdraw(&mut context, &test_pool_market, &user, 1)
        .await
        .unwrap();

    let (transit_account, _) = find_transit_program_address(
        &everlend_general_pool::id(),
        &test_pool_market.keypair.pubkey(),
        &test_pool.pool_mint.pubkey(),
    );

    assert_eq!(
        get_token_balance(&mut context, &user.pool_account).await,
        20
    );
    assert_eq!(
        get_token_balance(&mut context, &user.token_account).await,
        31
    );
    assert_eq!(
        get_token_balance(&mut context, &test_pool.token_account.pubkey()).await,
        70
    );

    assert_eq!(get_token_balance(&mut context, &transit_account).await, 50);
}
