#![cfg(feature = "test-bpf")]

use everlend_general_pool::find_transit_program_address;
use solana_program::pubkey::Pubkey;
use solana_program_test::*;
use solana_sdk::signer::Signer;

use crate::utils::*;

const INITIAL_USER_BALANCE: u64 = 5000000;

async fn setup(
    token_mint: Option<Pubkey>,
) -> (
    ProgramTestContext,
    TestGeneralPoolMarket,
    TestGeneralPool,
    TestGeneralPoolBorrowAuthority,
    LiquidityProvider,
) {
    let mut context = presetup().await.0;

    let test_pool_market = TestGeneralPoolMarket::new();
    test_pool_market.init(&mut context).await.unwrap();

    let test_pool = TestGeneralPool::new(&test_pool_market, token_mint);
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
    transfer(&mut context, &user.owner.pubkey(), INITIAL_USER_BALANCE)
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
    let (mut context, test_pool_market, test_pool, _pool_borrow_authority, user) =
        setup(None).await;

    test_pool
        .withdraw_request(&mut context, &test_pool_market, &user, 45)
        .await
        .unwrap();

    context.warp_to_slot(3).unwrap();

    let (collateral_transit_account, _) = find_transit_program_address(
        &everlend_general_pool::id(),
        &test_pool_market.keypair.pubkey(),
        &test_pool.pool_mint.pubkey(),
    );
    assert_eq!(
        get_token_balance(&mut context, &collateral_transit_account).await,
        45
    );

    test_pool
        .cancel_withdraw_request(&mut context, &test_pool_market, &user)
        .await
        .unwrap();

    assert_eq!(
        get_token_balance(&mut context, &user.pool_account).await,
        100
    );
    assert_eq!(
        get_token_balance(&mut context, &collateral_transit_account).await,
        0
    );

    let user_account = get_account(&mut context, &user.owner.pubkey()).await;
    assert_eq!(user_account.lamports, INITIAL_USER_BALANCE);
}
