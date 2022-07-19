use everlend_depositor::find_transit_program_address;
use everlend_liquidity_oracle::state::DistributionArray;
use everlend_registry::state::{
    DistributionPubkeys, RegistryRootAccounts, SetRegistryPoolConfigParams,
};
use everlend_utils::find_program_address;
use solana_program_test::*;
use solana_sdk::signer::Signer;

use crate::utils::*;

async fn setup() -> (
    ProgramTestContext,
    TestRegistry,
    TestGeneralPool,
    TestDepositor,
) {
    let (mut context, money_market, pyth_oracle, registry) = presetup().await;

    let payer_pubkey = context.payer.pubkey();

    // 0. Prepare lending
    let reserve = money_market.get_reserve_data(&mut context).await;
    println!("{:#?}", reserve);

    let collateral_mint = get_mint_data(&mut context, &reserve.collateral.mint_pubkey).await;
    println!("{:#?}", collateral_mint);

    // 1. Prepare general pool

    let general_pool_market = TestGeneralPoolMarket::new();
    general_pool_market
        .init(&mut context, &registry.keypair.pubkey())
        .await
        .unwrap();

    let general_pool = TestGeneralPool::new(&general_pool_market, None);
    general_pool
        .create(&mut context, &general_pool_market)
        .await
        .unwrap();
    registry
        .set_registry_pool_config(
            &mut context,
            &general_pool.pool_pubkey,
            SetRegistryPoolConfigParams {
                deposit_minimum: 0,
                withdraw_minimum: 0,
            },
        )
        .await
        .unwrap();

    // 1.1 Add liquidity to general pool

    let liquidity_provider = add_liquidity_provider(
        &mut context,
        &general_pool.token_mint_pubkey,
        &general_pool.pool_mint.pubkey(),
        9999 * EXP,
    )
    .await
    .unwrap();

    let mining_acc = general_pool.init_user_mining(&mut context, &general_pool_market, &liquidity_provider).await;

    general_pool
        .deposit(
            &mut context,
            &registry,
            &general_pool_market,
            &liquidity_provider,
            mining_acc,
            100 * EXP,
        )
        .await
        .unwrap();

    // 2. Prepare income pool
    let income_pool_market = TestIncomePoolMarket::new();
    income_pool_market
        .init(&mut context, &general_pool_market)
        .await
        .unwrap();

    let income_pool = TestIncomePool::new(&income_pool_market, None);
    income_pool
        .create(&mut context, &income_pool_market)
        .await
        .unwrap();

    // 3. Prepare money market pool

    let mm_pool_market = TestPoolMarket::new();
    mm_pool_market.init(&mut context).await.unwrap();

    let mm_pool = TestPool::new(&mm_pool_market, Some(reserve.collateral.mint_pubkey));
    mm_pool.create(&mut context, &mm_pool_market).await.unwrap();

    // 4. Prepare depositor

    // 4.1. Prepare liquidity oracle

    let test_liquidity_oracle = TestLiquidityOracle::new();
    test_liquidity_oracle.init(&mut context).await.unwrap();

    let mut distribution = DistributionArray::default();
    distribution[0] = 500_000_000u64; // 50%

    let test_token_distribution =
        TestTokenDistribution::new(general_pool.token_mint_pubkey, distribution);

    test_token_distribution
        .init(&mut context, &test_liquidity_oracle, payer_pubkey)
        .await
        .unwrap();

    test_token_distribution
        .update(
            &mut context,
            &test_liquidity_oracle,
            payer_pubkey,
            distribution,
        )
        .await
        .unwrap();

    let test_depositor = TestDepositor::new();
    test_depositor.init(&mut context, &registry).await.unwrap();

    // 4.2 Create transit account for liquidity token
    test_depositor
        .create_transit(&mut context, &general_pool.token_mint_pubkey, None)
        .await
        .unwrap();

    // 4.2.1 Create reserve transit account for liquidity token
    test_depositor
        .create_transit(
            &mut context,
            &general_pool.token_mint_pubkey,
            Some("reserve".to_string()),
        )
        .await
        .unwrap();
    let (reserve_transit_pubkey, _) = find_transit_program_address(
        &everlend_depositor::id(),
        &test_depositor.depositor.pubkey(),
        &general_pool.token_mint_pubkey,
        "reserve",
    );
    token_transfer(
        &mut context,
        &liquidity_provider.token_account,
        &reserve_transit_pubkey,
        &liquidity_provider.owner,
        10000,
    )
    .await
    .unwrap();

    // 4.3 Create transit account for collateral token
    test_depositor
        .create_transit(&mut context, &mm_pool.token_mint_pubkey, None)
        .await
        .unwrap();

    // 5. Prepare borrow authority
    let (depositor_authority, _) = find_program_address(
        &everlend_depositor::id(),
        &test_depositor.depositor.pubkey(),
    );
    let general_pool_borrow_authority =
        TestGeneralPoolBorrowAuthority::new(&general_pool, depositor_authority);
    general_pool_borrow_authority
        .create(
            &mut context,
            &general_pool_market,
            &general_pool,
            COLLATERAL_POOL_SHARE_ALLOWED,
        )
        .await
        .unwrap();

    let ten = [1, 2, 3, 4, 5, 6, 7, 8, 9, 0];
    let collateral_pool_markets = ten.map(|_| mm_pool_market.keypair.pubkey().clone());
    let mut roots = RegistryRootAccounts {
        general_pool_market: general_pool_market.keypair.pubkey(),
        income_pool_market: income_pool_market.keypair.pubkey(),
        liquidity_oracle: test_liquidity_oracle.keypair.pubkey(),
        collateral_pool_markets,
    };
    roots.collateral_pool_markets[0] = mm_pool_market.keypair.pubkey();
    registry
        .set_registry_root_accounts(&mut context, roots)
        .await
        .unwrap();

    // 6. Prepare withdraw authority
    let withdraw_authority = TestPoolWithdrawAuthority::new(&mm_pool, &depositor_authority);
    withdraw_authority
        .create(
            &mut context,
            &mm_pool_market,
            &mm_pool,
            &depositor_authority,
        )
        .await
        .unwrap();

    test_depositor
        .start_rebalancing(
            &mut context,
            &registry,
            &general_pool_market,
            &general_pool,
            &test_liquidity_oracle,
            false,
        )
        .await
        .unwrap();

    (context, registry, general_pool, test_depositor)
}

#[tokio::test]
async fn success() {
    let (mut context, registry, general_pool, test_depositor) = setup().await;

    test_depositor
        .reset_rebalancing(
            &mut context,
            &registry,
            &general_pool.token_mint_pubkey,
            100,
            DistributionArray::default(),
        )
        .await
        .unwrap();
}
