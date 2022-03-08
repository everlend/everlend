#![cfg(feature = "test-bpf")]

use crate::utils::*;
use everlend_general_pool::{find_transit_program_address, state::WithdrawalRequest};
use solana_program::clock::Slot;
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

    transfer(&mut context, &user.owner.pubkey(), 5000000)
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
        .withdraw_request(&mut context, &test_pool_market, &user, 50, 1)
        .await
        .unwrap();

    let withdraw_requests = test_pool
        .get_withdraw_requests(
            &mut context,
            &test_pool_market,
            &everlend_general_pool::id(),
        )
        .await;
    let (transit_account, _) = find_transit_program_address(
        &everlend_general_pool::id(),
        &test_pool_market.keypair.pubkey(),
        &test_pool.pool_mint.pubkey(),
    );
    let withdraw_request = test_pool
        .get_user_withdraw_requests(
            &mut context,
            &test_pool_market,
            1,
            &everlend_general_pool::id(),
        )
        .await;

    assert_eq!(
        get_token_balance(&mut context, &user.pool_account).await,
        50
    );
    assert_eq!(get_token_balance(&mut context, &transit_account).await, 50);
    assert_eq!(withdraw_requests.last_request_id, 1,);
    assert_eq!(withdraw_requests.liquidity_supply, 50);
    assert_eq!(
        withdraw_request,
        WithdrawalRequest {
            rent_payer: user.owner.pubkey(),
            source: user.pool_account,
            destination: user.token_account,
            liquidity_amount: 50,
            collateral_amount: 50,
        }
    );
}

const WARP_SLOT: Slot = 3;
#[tokio::test]
async fn success_few_requests() {
    let (mut context, test_pool_market, test_pool, _pool_borrow_authority, user) = setup().await;

    test_pool
        .deposit(&mut context, &test_pool_market, &user, 100)
        .await
        .unwrap();

    test_pool
        .withdraw_request(&mut context, &test_pool_market, &user, 50, 1)
        .await
        .unwrap();

    context.warp_to_slot(WARP_SLOT + 5).unwrap();

    test_pool
        .withdraw_request(&mut context, &test_pool_market, &user, 10, 2)
        .await
        .unwrap();

    context.warp_to_slot(WARP_SLOT + 9).unwrap();

    let withdraw_requests = test_pool
        .get_withdraw_requests(
            &mut context,
            &test_pool_market,
            &everlend_general_pool::id(),
        )
        .await;
    let (transit_account, _) = find_transit_program_address(
        &everlend_general_pool::id(),
        &test_pool_market.keypair.pubkey(),
        &test_pool.pool_mint.pubkey(),
    );
    let withdraw_request_1 = test_pool
        .get_user_withdraw_requests(
            &mut context,
            &test_pool_market,
            1,
            &everlend_general_pool::id(),
        )
        .await;
    let withdraw_request_2 = test_pool
        .get_user_withdraw_requests(
            &mut context,
            &test_pool_market,
            2,
            &everlend_general_pool::id(),
        )
        .await;

    assert_eq!(
        get_token_balance(&mut context, &user.pool_account).await,
        40
    );

    assert_eq!(get_token_balance(&mut context, &transit_account).await, 60);

    assert_eq!(withdraw_requests.last_request_id, 2);

    assert_eq!(withdraw_requests.liquidity_supply, 60);

    assert_eq!(
        withdraw_request_1,
        WithdrawalRequest {
            rent_payer: user.owner.pubkey(),
            source: user.pool_account,
            destination: user.token_account,
            liquidity_amount: 50,
            collateral_amount: 50,
        },
    );

    assert_eq!(
        withdraw_request_2,
        WithdrawalRequest {
            rent_payer: user.owner.pubkey(),
            source: user.pool_account,
            destination: user.token_account,
            liquidity_amount: 10,
            collateral_amount: 10,
        },
    );
}
