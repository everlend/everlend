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
    Pubkey,
) {
    let mut env = presetup().await;

    let test_pool_market = TestGeneralPoolMarket::new();
    test_pool_market
        .init(&mut env.context, &env.registry.keypair.pubkey())
        .await
        .unwrap();

    let test_pool = TestGeneralPool::new(&test_pool_market, token_mint);
    test_pool
        .create(&mut env.context, &test_pool_market)
        .await
        .unwrap();

    let test_pool_borrow_authority =
        TestGeneralPoolBorrowAuthority::new(&test_pool, env.context.payer.pubkey());
    test_pool_borrow_authority
        .create(
            &mut env.context,
            &test_pool_market,
            &test_pool,
            COLLATERAL_POOL_SHARE_ALLOWED,
        )
        .await
        .unwrap();

    let user = add_liquidity_provider(
        &mut env.context,
        &test_pool.token_mint_pubkey,
        &test_pool.pool_mint.pubkey(),
        101,
    )
    .await
    .unwrap();

    // Fill user account by native token
    transfer(&mut env.context, &user.owner.pubkey(), INITIAL_USER_BALANCE)
        .await
        .unwrap();

    let mining_acc = test_pool
        .init_user_mining(&mut env.context, &test_pool_market, &user)
        .await;

    test_pool
        .deposit(&mut env.context, &test_pool_market, &user, mining_acc, 100)
        .await
        .unwrap();

    (
        env.context,
        test_pool_market,
        test_pool,
        test_pool_borrow_authority,
        user,
        mining_acc,
    )
}

#[tokio::test]
async fn success() {
    let (mut context, test_pool_market, test_pool, _pool_borrow_authority, user, mining_acc) =
        setup(None).await;

    test_pool
        .withdraw_request(&mut context, &test_pool_market, &user, mining_acc, 45)
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
