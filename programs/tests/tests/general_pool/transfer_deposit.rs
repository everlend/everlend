use crate::utils::*;
use everlend_general_pool::{find_transit_program_address};
use solana_program::instruction::InstructionError;
use solana_program::pubkey::Pubkey;
use solana_program_test::*;
use solana_sdk::signer::Signer;
use solana_sdk::transaction::TransactionError;

async fn setup() -> (
    ProgramTestContext,
    TestGeneralPoolMarket,
    TestGeneralPool,
    TestGeneralPoolBorrowAuthority,
    LiquidityProvider,
    LiquidityProvider,
    Pubkey,
    Pubkey
) {
    let mut env = presetup().await;

    let test_pool_market = TestGeneralPoolMarket::new();
    test_pool_market
        .init(&mut env.context, &env.registry.keypair.pubkey())
        .await
        .unwrap();

    let test_pool = TestGeneralPool::new(&test_pool_market, None);
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
        200,
    )
        .await
        .unwrap();

    let destination_user = add_liquidity_provider(
        &mut env.context,
        &test_pool.token_mint_pubkey,
        &test_pool.pool_mint.pubkey(),
        200
    )
        .await
        .unwrap();

    transfer(&mut env.context, &user.owner.pubkey(), 5000000)
        .await
        .unwrap();
    transfer(&mut env.context, &destination_user.owner.pubkey(), 5000000)
        .await
        .unwrap();

    let mining_acc = test_pool
        .init_user_mining(&mut env.context, &test_pool_market, &user)
        .await;
    let destination_mining_acc =
        test_pool
            .init_user_mining(&mut env.context, &test_pool_market, &destination_user)
            .await;

    (
        env.context,
        test_pool_market,
        test_pool,
        test_pool_borrow_authority,
        user,
        destination_user,
        mining_acc,
        destination_mining_acc,
    )
}

#[tokio::test]
async fn success() {
    let (
        mut context,
        test_pool_market,
        test_pool,
        _pool_borrow_authority,
        user,
        destination_user,
        mining_acc,
        destination_mining_acc,
    ) = setup().await;

    test_pool
        .deposit(
            &mut context,
            &test_pool_market,
            &user,
            mining_acc,
            100,
        )
        .await
        .unwrap();

    context.warp_to_slot(3).unwrap();

    assert_eq!(
        get_token_balance(&mut context, &user.pool_account).await,
        100
    );

    test_pool
        .transfer_deposit(
            &mut context,
            &user,
            &destination_user,
            mining_acc,
            destination_mining_acc,
        )
        .await
        .unwrap();

    assert_eq!(
        get_token_balance(&mut context, &user.pool_account).await,
        0
    );
    assert_eq!(get_token_balance(&mut context, &destination_user.pool_account).await, 100);
}

#[tokio::test]
async fn failed_after_spl_transfer() {
    let (
        mut context,
        test_pool_market,
        test_pool,
        _pool_borrow_authority,
        user,
        destination_user,
        mining_acc,
        destination_mining_acc,
    ) = setup().await;

    test_pool
        .deposit(
            &mut context,
            &test_pool_market,
            &user,
            mining_acc,
            100,
        )
        .await
        .unwrap();
    test_pool
        .deposit(
            &mut context,
            &test_pool_market,
            &destination_user,
            destination_mining_acc,
            100,
        )
        .await
        .unwrap();

    context.warp_to_slot(3).unwrap();

    assert_eq!(
        get_token_balance(&mut context, &user.pool_account).await,
        100
    );

    token_transfer(
        &mut context,
        &user.pool_account,
        &destination_user.pool_account,
        &user.owner,
        50,
    )
        .await
        .unwrap();

    assert_eq!(
        get_token_balance(&mut context, &user.pool_account).await,
        50
    );
    assert_eq!(
        get_token_balance(&mut context, &destination_user.pool_account).await,
        150
    );

    assert_eq!(
        test_pool
            .transfer_deposit(
                &mut context,
                &destination_user,
                &user,
                destination_mining_acc,
                mining_acc,
            )
            .await.unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(6003 as u32) // MathOverflow error
        )
    )
}


#[tokio::test]
async fn successful_withdraw_request_after_transfer() {
    let (
        mut context,
        test_pool_market,
        test_pool,
        _pool_borrow_authority,
        user,
        destination_user,
        mining_acc,
        destination_mining_acc,
    ) = setup().await;

    test_pool
        .deposit(
            &mut context,
            &test_pool_market,
            &user,
            mining_acc,
            100,
        )
        .await
        .unwrap();

    context.warp_to_slot(3).unwrap();

    test_pool
        .transfer_deposit(
            &mut context,
            &user,
            &destination_user,
            mining_acc,
            destination_mining_acc,
        )
        .await
        .unwrap();

    test_pool.withdraw_request(
        &mut context,
        &test_pool_market,
        &destination_user,
        destination_mining_acc,
        50,
    ).await.unwrap();

    let (withdraw_requests_pubkey, withdraw_requests) = test_pool
        .get_withdrawal_requests(&mut context, &test_pool_market)
        .await;
    let (transit_account, _) = find_transit_program_address(
        &everlend_general_pool::id(),
        &test_pool_market.keypair.pubkey(),
        &test_pool.pool_mint.pubkey(),
    );
    let withdraw_request = test_pool
        .get_withdrawal_request(&mut context, &withdraw_requests_pubkey, &destination_user.pubkey())
        .await;

    assert_eq!(
        get_token_balance(&mut context, &user.pool_account).await,
        0
    );
    assert_eq!(get_token_balance(&mut context, &transit_account).await, 50);
    assert_eq!(get_token_balance(&mut context, &destination_user.pool_account).await, 50);
    assert_eq!(withdraw_requests.liquidity_supply, 50);
    assert_eq!(withdraw_request.liquidity_amount, 50);
}
