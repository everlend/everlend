#![cfg(feature = "test-bpf")]

use crate::utils::*;
use everlend_utils::find_program_address;
use solana_program::{program_pack::Pack, pubkey::Pubkey};
use solana_program_test::*;
use solana_sdk::{signature::Keypair, signer::Signer};

async fn setup() -> (
    ProgramTestContext,
    TestSPLTokenLending,
    TestPoolMarket,
    TestPool,
    TestPoolBorrowAuthority,
    TestPoolMarket,
    TestPool,
    LiquidityProvider,
    TestDepositor,
) {
    let (mut context, spl_token_lending, _) = presetup().await;

    let owner = Keypair::new();

    transfer(&mut context, &owner.pubkey(), 999999999)
        .await
        .unwrap();

    // 0. Prepare lending
    let reserve = get_reserve_account_data(&mut context, &spl_token_lending.reserve_pubkey).await;
    println!("{:#?}", reserve);

    let account = get_account(&mut context, &spl_token_lending.market_pubkey).await;
    let lending_market =
        spl_token_lending::state::LendingMarket::unpack_from_slice(account.data.as_slice())
            .unwrap();

    let authority_signer_seeds = &[
        &spl_token_lending.market_pubkey.to_bytes()[..32],
        &[lending_market.bump_seed],
    ];
    let lending_market_authority_pubkey =
        Pubkey::create_program_address(authority_signer_seeds, &spl_token_lending::id()).unwrap();

    println!("{:#?}", lending_market_authority_pubkey);

    let collateral_mint = get_mint_data(&mut context, &reserve.collateral.mint_pubkey).await;
    println!("{:#?}", collateral_mint);

    // 1. Prepare general pool

    let general_pool_market = TestPoolMarket::new();
    general_pool_market.init(&mut context).await.unwrap();

    let general_pool = TestPool::new(&general_pool_market, None);
    general_pool
        .create(&mut context, &general_pool_market)
        .await
        .unwrap();

    // let borrow_authority = Keypair::from_bytes(&owner.to_bytes()).unwrap();

    // 1.1 Add liquidity to general pool

    let liquidity_provider = add_liquidity_provider(&mut context, &general_pool, 9999 * EXP)
        .await
        .unwrap();

    general_pool
        .deposit(
            &mut context,
            &general_pool_market,
            &liquidity_provider,
            100 * EXP,
        )
        .await
        .unwrap();

    // 2. Prepare money market pool

    let mm_pool_market = TestPoolMarket::new();
    mm_pool_market.init(&mut context).await.unwrap();

    let mm_pool = TestPool::new(&mm_pool_market, Some(reserve.collateral.mint_pubkey));
    mm_pool.create(&mut context, &mm_pool_market).await.unwrap();

    // 3. Prepare depositor

    let test_depositor = TestDepositor::new();
    test_depositor.init(&mut context).await.unwrap();

    // 3.1 Create transit account for liquidity token
    test_depositor
        .create_transit(&mut context, &general_pool.token_mint_pubkey)
        .await
        .unwrap();

    // 3.2 Create transit account for collateral token
    test_depositor
        .create_transit(&mut context, &mm_pool.token_mint_pubkey)
        .await
        .unwrap();

    // 4. Prepare borrow authority
    let (depositor_authority, _) = find_program_address(
        &everlend_depositor::id(),
        &test_depositor.depositor.pubkey(),
    );
    let general_pool_borrow_authority =
        TestPoolBorrowAuthority::new(&general_pool, depositor_authority);
    general_pool_borrow_authority
        .create(
            &mut context,
            &general_pool_market,
            &general_pool,
            SHARE_ALLOWED,
        )
        .await
        .unwrap();

    (
        context,
        spl_token_lending,
        general_pool_market,
        general_pool,
        general_pool_borrow_authority,
        mm_pool_market,
        mm_pool,
        liquidity_provider,
        test_depositor,
    )
}

#[tokio::test]
async fn success() {
    let (
        mut context,
        spl_token_lending,
        general_pool_market,
        general_pool,
        general_pool_borrow_authority,
        mm_pool_market,
        mm_pool,
        liquidity_provider,
        test_depositor,
    ) = setup().await;

    test_depositor
        .deposit(
            &mut context,
            &general_pool_market,
            &general_pool,
            &general_pool_borrow_authority,
            &mm_pool_market,
            &mm_pool,
            &spl_token_lending,
            100,
        )
        .await
        .unwrap();

    // assert_eq!(
    //     get_token_balance(&mut context, &user.destination).await,
    //     100,
    // );
}
