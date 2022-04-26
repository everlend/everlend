#![cfg(feature = "test-bpf")]

use solana_program::instruction::InstructionError;
use solana_program::pubkey::Pubkey;
use solana_program_test::*;
use solana_sdk::signature::Keypair;
use solana_sdk::transaction::Transaction;
use solana_sdk::{signer::Signer, transaction::TransactionError};

use everlend_depositor::find_transit_program_address;
use everlend_liquidity_oracle::state::DistributionArray;
use everlend_registry::state::{PoolMarketsConfig, SetRegistryConfigParams, TOTAL_DISTRIBUTIONS};
use everlend_utils::{
    find_program_address,
    integrations::{self, MoneyMarketPubkeys},
    EverlendError,
};

use crate::utils::*;

async fn setup() -> (
    ProgramTestContext,
    TestSPLTokenLending,
    TestPythOracle,
    TestRegistry,
    TestGeneralPoolMarket,
    TestGeneralPool,
    TestGeneralPoolBorrowAuthority,
    TestIncomePoolMarket,
    TestIncomePool,
    TestUlpPoolMarket,
    TestPool,
    LiquidityProvider,
    TestDepositor,
    TestLiquidityOracle,
    TestTokenDistribution,
    DistributionArray,
) {
    let (
        mut context,
        money_market,
        pyth_oracle,
        registry,
        general_pool_market,
        income_pool_market,
        mm_pool_market,
    ) = presetup().await;

    let payer_pubkey = context.payer.pubkey();

    // Prepare lending
    let reserve = money_market.get_reserve_data(&mut context).await;
    println!("{:#?}", reserve);

    let collateral_mint = get_mint_data(&mut context, &reserve.collateral.mint_pubkey).await;
    println!("{:#?}", collateral_mint);

    // Prepare general pool

    let general_pool = TestGeneralPool::new(&general_pool_market, None);
    general_pool
        .create(&mut context, &general_pool_market)
        .await
        .unwrap();

    // Add liquidity to general pool

    let liquidity_provider = add_liquidity_provider(
        &mut context,
        &general_pool.token_mint_pubkey,
        &general_pool.pool_mint.pubkey(),
        9999 * EXP,
    )
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

    // Prepare income pool

    let income_pool = TestIncomePool::new(&income_pool_market, None);
    income_pool
        .create(&mut context, &income_pool_market)
        .await
        .unwrap();

    // Prepare money market pool

    let mm_pool = TestPool::new(&mm_pool_market, Some(reserve.collateral.mint_pubkey));
    mm_pool.create(&mut context, &mm_pool_market).await.unwrap();

    // Prepare depositor:
    // Prepare liquidity oracle

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
    test_depositor
        .init(
            &mut context,
            &registry,
            &general_pool_market,
            &income_pool_market,
            &test_liquidity_oracle,
        )
        .await
        .unwrap();

    // Create transit account for liquidity token
    test_depositor
        .create_transit(&mut context, &general_pool.token_mint_pubkey, None)
        .await
        .unwrap();

    // Create reserve transit account for liquidity token
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

    // Create transit account for collateral token
    test_depositor
        .create_transit(&mut context, &mm_pool.token_mint_pubkey, None)
        .await
        .unwrap();

    // Create transit account for mm pool collateral token
    test_depositor
        .create_transit(&mut context, &mm_pool.pool_mint.pubkey(), None)
        .await
        .unwrap();

    // Prepare borrow authority
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
            ULP_SHARE_ALLOWED,
        )
        .await
        .unwrap();

    (
        context,
        money_market,
        pyth_oracle,
        registry,
        general_pool_market,
        general_pool,
        general_pool_borrow_authority,
        income_pool_market,
        income_pool,
        mm_pool_market,
        mm_pool,
        liquidity_provider,
        test_depositor,
        test_liquidity_oracle,
        test_token_distribution,
        distribution,
    )
}

#[tokio::test]
async fn success() {
    let (
        mut context,
        _,
        _,
        registry,
        general_pool_market,
        general_pool,
        _,
        _,
        _,
        _,
        _,
        _,
        test_depositor,
        test_liquidity_oracle,
        _,
        _,
    ) = setup().await;

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
}

#[tokio::test]
async fn success_with_refresh_income() {
    let (
        mut context,
        money_market,
        pyth_oracle,
        registry,
        general_pool_market,
        general_pool,
        _,
        income_pool_market,
        income_pool,
        mm_pool_market,
        mm_pool,
        liquidity_provider,
        test_depositor,
        test_liquidity_oracle,
        test_token_distribution,
        mut distribution,
    ) = setup().await;
    let payer_pubkey = context.payer.pubkey();
    let reserve = money_market.get_reserve_data(&mut context).await;
    let money_market_pubkeys =
        MoneyMarketPubkeys::SPL(integrations::spl_token_lending::AccountPubkeys {
            reserve: money_market.reserve_pubkey,
            reserve_liquidity_supply: reserve.liquidity.supply_pubkey,
            reserve_liquidity_oracle: reserve.liquidity.oracle_pubkey,
            lending_market: money_market.market_pubkey,
        });

    // Start rebalancing
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

    // Rates should be refreshed
    context.warp_to_slot(REFRESH_INCOME_INTERVAL).unwrap();
    pyth_oracle
        .update(&mut context, REFRESH_INCOME_INTERVAL)
        .await;

    test_depositor
        .deposit(
            &mut context,
            &registry,
            &mm_pool_market,
            &mm_pool,
            &spl_token_lending::id(),
            &money_market_pubkeys,
        )
        .await
        .unwrap();

    distribution[0] = 0;
    test_token_distribution
        .update(
            &mut context,
            &test_liquidity_oracle,
            payer_pubkey,
            distribution,
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
            true,
        )
        .await
        .unwrap();

    let rebalancing = test_depositor
        .get_rebalancing_data(&mut context, &general_pool.token_mint_pubkey)
        .await;
    println!("rebalancing = {:#?}", rebalancing);

    let mut reserve = money_market.get_reserve_data(&mut context).await;
    // Transfer some tokens to liquidity account to get incomes
    token_transfer(
        &mut context,
        &liquidity_provider.token_account,
        &reserve.liquidity.supply_pubkey,
        &liquidity_provider.owner,
        10 * EXP,
    )
    .await
    .unwrap();
    reserve.liquidity.deposit(10 * EXP).unwrap();
    money_market.update_reserve(&mut context, &reserve).await;

    // Rates should be refreshed
    context.warp_to_slot(REFRESH_INCOME_INTERVAL + 5).unwrap();
    pyth_oracle
        .update(&mut context, REFRESH_INCOME_INTERVAL + 5)
        .await;

    test_depositor
        .withdraw(
            &mut context,
            &registry,
            &income_pool_market,
            &income_pool,
            &mm_pool_market,
            &mm_pool,
            &spl_token_lending::id(),
            &money_market_pubkeys,
        )
        .await
        .unwrap();

    test_depositor
        .deposit(
            &mut context,
            &registry,
            &mm_pool_market,
            &mm_pool,
            &spl_token_lending::id(),
            &money_market_pubkeys,
        )
        .await
        .unwrap();

    let income_balance = get_token_balance(&mut context, &income_pool.token_account.pubkey()).await;
    println!("Income balance: {}", income_balance);
    assert!(income_balance > 0);
}

#[tokio::test]
async fn fail_with_already_refreshed_income() {
    let (
        mut context,
        money_market,
        pyth_oracle,
        registry,
        general_pool_market,
        general_pool,
        _,
        _,
        _,
        mm_pool_market,
        mm_pool,
        _,
        test_depositor,
        test_liquidity_oracle,
        test_token_distribution,
        mut distribution,
    ) = setup().await;
    let payer_pubkey = context.payer.pubkey();
    let reserve = money_market.get_reserve_data(&mut context).await;
    let money_market_pubkeys =
        MoneyMarketPubkeys::SPL(integrations::spl_token_lending::AccountPubkeys {
            reserve: money_market.reserve_pubkey,
            reserve_liquidity_supply: reserve.liquidity.supply_pubkey,
            reserve_liquidity_oracle: reserve.liquidity.oracle_pubkey,
            lending_market: money_market.market_pubkey,
        });

    // Start rebalancing
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

    // Rates should be refreshed
    context.warp_to_slot(REFRESH_INCOME_INTERVAL - 1).unwrap();
    pyth_oracle
        .update(&mut context, REFRESH_INCOME_INTERVAL - 1)
        .await;

    test_depositor
        .deposit(
            &mut context,
            &registry,
            &mm_pool_market,
            &mm_pool,
            &spl_token_lending::id(),
            &money_market_pubkeys,
        )
        .await
        .unwrap();

    distribution[0] = 0;
    test_token_distribution
        .update(
            &mut context,
            &test_liquidity_oracle,
            payer_pubkey,
            distribution,
        )
        .await
        .unwrap();

    assert_eq!(
        test_depositor
            .start_rebalancing(
                &mut context,
                &registry,
                &general_pool_market,
                &general_pool,
                &test_liquidity_oracle,
                true,
            )
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(EverlendError::IncomeRefreshed as u32),
        )
    );
}

#[tokio::test]
async fn fail_with_invalid_registry() {
    let (
        mut context,
        _,
        _,
        _,
        general_pool_market,
        general_pool,
        _,
        income_pool_market,
        _,
        mm_pool_market,
        _,
        _,
        test_depositor,
        test_liquidity_oracle,
        _,
        _,
    ) = setup().await;

    let refresh_income = false;

    // Prepare wrong config registry

    let wrong_registry = {
        let wrong_registry = TestRegistry::new_with_manager(
            Keypair::from_bytes(context.payer.to_bytes().as_ref()).unwrap(),
        );
        wrong_registry.init(&mut context).await.unwrap();

        let config = SetRegistryConfigParams {
            general_pool_program_id: everlend_general_pool::id(),
            ulp_program_id: everlend_ulp::id(),
            liquidity_oracle_program_id: everlend_liquidity_oracle::id(),
            depositor_program_id: everlend_depositor::id(),
            income_pools_program_id: everlend_income_pools::id(),
            money_market_program_ids: [spl_token_lending::id(); TOTAL_DISTRIBUTIONS],
            refresh_income_interval: REFRESH_INCOME_INTERVAL,
        };

        let mut ulp_pool_markets = [Pubkey::default(); TOTAL_DISTRIBUTIONS];
        ulp_pool_markets[0] = mm_pool_market.keypair.pubkey();

        let pool_markets_cfg = PoolMarketsConfig {
            general_pool_market: general_pool_market.keypair.pubkey(),
            income_pool_market: income_pool_market.keypair.pubkey(),
            ulp_pool_markets,
        };

        wrong_registry
            .set_registry_config(&mut context, config, pool_markets_cfg)
            .await
            .unwrap();

        wrong_registry.keypair.pubkey()
    };

    let tx = Transaction::new_signed_with_payer(
        &[everlend_depositor::instruction::start_rebalancing(
            &everlend_depositor::id(),
            &wrong_registry,
            &test_depositor.depositor.pubkey(),
            &general_pool.token_mint_pubkey,
            &general_pool_market.keypair.pubkey(),
            &general_pool.token_account.pubkey(),
            &test_liquidity_oracle.keypair.pubkey(),
            &context.payer.pubkey(),
            refresh_income,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );

    assert_eq!(
        context
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(0, InstructionError::InvalidArgument)
    );
}

#[tokio::test]
async fn fail_with_invalid_depositor() {
    let (
        mut context,
        _,
        _,
        registry,
        general_pool_market,
        general_pool,
        _,
        _,
        _,
        _,
        _,
        _,
        _,
        test_liquidity_oracle,
        _,
        _,
    ) = setup().await;

    let refresh_income = false;

    let tx = Transaction::new_signed_with_payer(
        &[everlend_depositor::instruction::start_rebalancing(
            &everlend_depositor::id(),
            &registry.keypair.pubkey(),
            &Pubkey::new_unique(),
            &general_pool.token_mint_pubkey,
            &general_pool_market.keypair.pubkey(),
            &general_pool.token_account.pubkey(),
            &test_liquidity_oracle.keypair.pubkey(),
            &context.payer.pubkey(),
            refresh_income,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );

    assert_eq!(
        context
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(EverlendError::InvalidAccountOwner as u32),
        )
    );
}

#[tokio::test]
async fn fail_with_invalid_mint() {
    let (
        mut context,
        _,
        _,
        registry,
        general_pool_market,
        general_pool,
        _,
        _,
        _,
        _,
        _,
        _,
        test_depositor,
        test_liquidity_oracle,
        _,
        _,
    ) = setup().await;

    let refresh_income = false;

    let tx = Transaction::new_signed_with_payer(
        &[everlend_depositor::instruction::start_rebalancing(
            &everlend_depositor::id(),
            &registry.keypair.pubkey(),
            &test_depositor.depositor.pubkey(),
            &Pubkey::new_unique(),
            &general_pool_market.keypair.pubkey(),
            &general_pool.token_account.pubkey(),
            &test_liquidity_oracle.keypair.pubkey(),
            &context.payer.pubkey(),
            refresh_income,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );

    assert_eq!(
        context
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(EverlendError::InvalidAccountOwner as u32),
        )
    );
}

#[tokio::test]
async fn fail_with_invalid_general_pool_market() {
    let (
        mut context,
        _,
        _,
        registry,
        _,
        general_pool,
        _,
        _,
        _,
        _,
        _,
        _,
        test_depositor,
        test_liquidity_oracle,
        _,
        _,
    ) = setup().await;

    let refresh_income = false;

    let tx = Transaction::new_signed_with_payer(
        &[everlend_depositor::instruction::start_rebalancing(
            &everlend_depositor::id(),
            &registry.keypair.pubkey(),
            &test_depositor.depositor.pubkey(),
            &general_pool.token_mint_pubkey,
            &Pubkey::new_unique(),
            &general_pool.token_account.pubkey(),
            &test_liquidity_oracle.keypair.pubkey(),
            &context.payer.pubkey(),
            refresh_income,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );

    assert_eq!(
        context
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(EverlendError::InvalidAccountOwner as u32),
        )
    );
}

#[tokio::test]
async fn fail_with_invalid_general_pool_token_account() {
    let (
        mut context,
        _,
        _,
        registry,
        general_pool_market,
        general_pool,
        _,
        _,
        _,
        _,
        _,
        _,
        test_depositor,
        test_liquidity_oracle,
        _,
        _,
    ) = setup().await;

    let refresh_income = false;

    let tx = Transaction::new_signed_with_payer(
        &[everlend_depositor::instruction::start_rebalancing(
            &everlend_depositor::id(),
            &registry.keypair.pubkey(),
            &test_depositor.depositor.pubkey(),
            &general_pool.token_mint_pubkey,
            &general_pool_market.keypair.pubkey(),
            &Pubkey::new_unique(),
            &test_liquidity_oracle.keypair.pubkey(),
            &context.payer.pubkey(),
            refresh_income,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );

    assert_eq!(
        context
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(0, InstructionError::InvalidArgument)
    );
}

#[tokio::test]
async fn fail_with_invalid_liquidity_oracle() {
    let (
        mut context,
        _,
        _,
        registry,
        general_pool_market,
        general_pool,
        _,
        _,
        _,
        _,
        _,
        _,
        test_depositor,
        _,
        _,
        _,
    ) = setup().await;

    let refresh_income = false;

    let tx = Transaction::new_signed_with_payer(
        &[everlend_depositor::instruction::start_rebalancing(
            &everlend_depositor::id(),
            &registry.keypair.pubkey(),
            &test_depositor.depositor.pubkey(),
            &general_pool.token_mint_pubkey,
            &general_pool_market.keypair.pubkey(),
            &general_pool.token_account.pubkey(),
            &Pubkey::new_unique(),
            &context.payer.pubkey(),
            refresh_income,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );

    assert_eq!(
        context
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(EverlendError::InvalidAccountOwner as u32),
        )
    );
}
