use crate::utils::*;
use everlend_depositor::state::{Rebalancing, RebalancingOperation};
use everlend_depositor::utils::calculate_amount_to_distribute;
use everlend_liquidity_oracle::state::{DistributionArray, TokenOracle};
use everlend_registry::instructions::{UpdateRegistryData, UpdateRegistryMarketsData};
use everlend_registry::state::DistributionPubkeys;
use everlend_utils::{abs_diff, percent_ratio, PDA};
use everlend_utils::{
    find_program_address,
    integrations::{self, MoneyMarketPubkeys},
    EverlendError,
};
use solana_program::instruction::InstructionError;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program_test::*;
use solana_sdk::signature::Keypair;
use solana_sdk::transaction::Transaction;
use solana_sdk::{signer::Signer, transaction::TransactionError};
use std::vec;

async fn setup(
    deposit_amount: u64,
) -> (
    ProgramTestContext,
    TestSPLTokenLending,
    TestPythOracle,
    TestRegistry,
    TestGeneralPoolMarket,
    TestGeneralPool,
    TestGeneralPoolBorrowAuthority,
    TestIncomePoolMarket,
    TestIncomePool,
    TestPoolMarket,
    TestPool,
    LiquidityProvider,
    TestDepositor,
    TestLiquidityOracle,
    TestTokenOracle,
    DistributionArray,
) {
    let mut env = presetup().await;

    let payer_pubkey = env.context.payer.pubkey();

    // 0. Prepare lending
    let reserve = env
        .spl_token_lending
        .get_reserve_data(&mut env.context)
        .await;
    println!("{:#?}", reserve);

    let collateral_mint = get_mint_data(&mut env.context, &reserve.collateral.mint_pubkey).await;
    println!("{:#?}", collateral_mint);

    // 1. Prepare general pool

    let general_pool_market = TestGeneralPoolMarket::new();
    general_pool_market
        .init(&mut env.context, &env.registry.keypair.pubkey())
        .await
        .unwrap();

    let general_pool = TestGeneralPool::new(&general_pool_market, None);
    general_pool
        .create(&mut env.context, &general_pool_market)
        .await
        .unwrap();

    // 1.1 Add liquidity to general pool

    let liquidity_provider = add_liquidity_provider(
        &mut env.context,
        &general_pool.token_mint_pubkey,
        &general_pool.pool_mint.pubkey(),
        9999 * EXP,
    )
    .await
    .unwrap();

    let mining_acc = general_pool
        .init_user_mining(&mut env.context, &general_pool_market, &liquidity_provider)
        .await;

    general_pool
        .deposit(
            &mut env.context,
            &general_pool_market,
            &liquidity_provider,
            mining_acc,
            deposit_amount,
        )
        .await
        .unwrap();

    // 2. Prepare income pool
    let income_pool_market = TestIncomePoolMarket::new();
    income_pool_market
        .init(&mut env.context, &general_pool_market)
        .await
        .unwrap();

    let income_pool = TestIncomePool::new(&income_pool_market, None);
    income_pool
        .create(&mut env.context, &income_pool_market)
        .await
        .unwrap();

    // 3. Prepare money market pool

    let mm_pool_market = TestPoolMarket::new();
    mm_pool_market.init(&mut env.context).await.unwrap();

    let mm_pool = TestPool::new(&mm_pool_market, Some(reserve.collateral.mint_pubkey));
    mm_pool
        .create(&mut env.context, &mm_pool_market)
        .await
        .unwrap();

    // 4. Prepare depositor

    // 4.1. Prepare liquidity oracle

    let test_liquidity_oracle = TestLiquidityOracle::new();
    test_liquidity_oracle.init(&mut env.context).await.unwrap();

    let mut distribution = DistributionArray::default();
    distribution[0] = 500_000_000u64; // 50%

    let test_token_oracle = TestTokenOracle::new(general_pool.token_mint_pubkey, distribution);

    test_token_oracle
        .init(&mut env.context, &test_liquidity_oracle, payer_pubkey)
        .await
        .unwrap();

    test_token_oracle
        .update(
            &mut env.context,
            &test_liquidity_oracle,
            payer_pubkey,
            distribution,
        )
        .await
        .unwrap();

    let test_depositor = TestDepositor::new();
    test_depositor
        .init(&mut env.context, &env.registry)
        .await
        .unwrap();

    // 4.2 Create transit account for liquidity token
    test_depositor
        .create_transit(&mut env.context, &general_pool.token_mint_pubkey, None)
        .await
        .unwrap();

    // 4.2.1 Create reserve transit account for liquidity token
    test_depositor
        .create_transit(
            &mut env.context,
            &general_pool.token_mint_pubkey,
            Some("reserve".to_string()),
        )
        .await
        .unwrap();
    let (reserve_transit_pubkey, _) = everlend_depositor::TransitPDA {
        seed: "reserve",
        depositor: test_depositor.depositor.pubkey(),
        mint: general_pool.token_mint_pubkey,
    }
    .find_address(&everlend_depositor::id());
    token_transfer(
        &mut env.context,
        &liquidity_provider.token_account,
        &reserve_transit_pubkey,
        &liquidity_provider.owner,
        10000,
    )
    .await
    .unwrap();

    // 4.3 Create transit account for collateral token
    test_depositor
        .create_transit(&mut env.context, &mm_pool.token_mint_pubkey, None)
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
            &mut env.context,
            &general_pool_market,
            &general_pool,
            COLLATERAL_POOL_SHARE_ALLOWED,
        )
        .await
        .unwrap();

    let ten = [1, 2, 3, 4, 5, 6, 7, 8, 9, 0];
    let mut collateral_pool_markets = ten.map(|_| mm_pool_market.keypair.pubkey().clone());
    collateral_pool_markets[0] = mm_pool_market.keypair.pubkey();

    env.registry
        .update_registry(
            &mut env.context,
            UpdateRegistryData {
                general_pool_market: Some(general_pool_market.keypair.pubkey()),
                income_pool_market: Some(income_pool_market.keypair.pubkey()),
                liquidity_oracle: Some(test_liquidity_oracle.keypair.pubkey()),
                refresh_income_interval: None,
            },
        )
        .await
        .unwrap();

    env.registry
        .update_registry_markets(
            &mut env.context,
            UpdateRegistryMarketsData {
                money_markets: None,
                collateral_pool_markets: Some(collateral_pool_markets),
            },
        )
        .await
        .unwrap();

    // 6. Prepare withdraw authority
    let withdraw_authority = TestPoolWithdrawAuthority::new(&mm_pool, &depositor_authority);
    withdraw_authority
        .create(
            &mut env.context,
            &mm_pool_market,
            &mm_pool,
            &depositor_authority,
        )
        .await
        .unwrap();

    (
        env.context,
        env.spl_token_lending,
        env.pyth_oracle,
        env.registry,
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
        test_token_oracle,
        distribution,
    )
}

#[tokio::test]
async fn success() {
    let deposit_amount = 100 * EXP;
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
    ) = setup(deposit_amount).await;

    test_depositor
        .start_rebalancing(
            &mut context,
            &registry,
            &general_pool_market,
            &general_pool,
            &test_liquidity_oracle,
            false,
            DistributionArray::default(),
        )
        .await
        .unwrap();

    let data = test_depositor
        .get_rebalancing_data(&mut context, &general_pool.token_mint_pubkey)
        .await;

    assert_eq!(
        data.total_distributed_liquidity().unwrap(),
        deposit_amount / 2
    ); // 50% distrubution
    assert_eq!(data.amount_to_distribute, deposit_amount);
}

#[tokio::test]
async fn success_with_reserve_rates() {
    let deposit_amount = 10;
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
    ) = setup(deposit_amount).await;

    let mut reserve_rates = DistributionArray::default();
    reserve_rates[0] = 10_000_000; // 1% ratio - really low rate just for test

    test_depositor
        .start_rebalancing(
            &mut context,
            &registry,
            &general_pool_market,
            &general_pool,
            &test_liquidity_oracle,
            false,
            reserve_rates,
        )
        .await
        .unwrap();

    let data = test_depositor
        .get_rebalancing_data(&mut context, &general_pool.token_mint_pubkey)
        .await;

    assert_eq!(data.total_distributed_liquidity().unwrap(), 0); // because of collateral ratio
    assert_eq!(data.amount_to_distribute, deposit_amount);
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
        test_token_oracle,
        mut distribution,
    ) = setup(100 * EXP).await;
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
            DistributionArray::default(),
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
    test_token_oracle
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
            DistributionArray::default(),
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
        .refresh_mm_incomes(
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

    let income_balance = get_token_balance(&mut context, &income_pool.token_account.pubkey()).await;
    println!("Income balance: {}", income_balance);
    assert!(income_balance > 0);
}

#[tokio::test]
async fn success_with_skipped_refresh_steps() {
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
        _,
        test_depositor,
        test_liquidity_oracle,
        test_token_oracle,
        mut distribution,
    ) = setup(100 * EXP).await;
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
            DistributionArray::default(),
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
    test_token_oracle
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
            DistributionArray::default(),
        )
        .await
        .unwrap();

    // Rates should be refreshed
    context.warp_to_slot(REFRESH_INCOME_INTERVAL + 5).unwrap();
    pyth_oracle
        .update(&mut context, REFRESH_INCOME_INTERVAL + 5)
        .await;

    test_depositor
        .refresh_mm_incomes(
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

    // Refresh steps should be skipped and marked as executed
    let rebalancing = test_depositor
        .get_rebalancing_data(&mut context, &general_pool.token_mint_pubkey)
        .await;

    assert_eq!(
        rebalancing.steps[0].executed_at,
        Some(REFRESH_INCOME_INTERVAL + 5)
    );
    assert_eq!(
        rebalancing.steps[1].executed_at,
        Some(REFRESH_INCOME_INTERVAL + 5)
    );

    let income_balance = get_token_balance(&mut context, &income_pool.token_account.pubkey()).await;
    assert_eq!(income_balance, 0);
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
        test_token_oracle,
        mut distribution,
    ) = setup(100 * EXP).await;
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
            DistributionArray::default(),
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
    test_token_oracle
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
                DistributionArray::default(),
            )
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(
            1,
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
        _,
        _,
        _,
        _,
        _,
        test_depositor,
        test_liquidity_oracle,
        _,
        _,
    ) = setup(100 * EXP).await;

    let refresh_income = false;

    let tx = Transaction::new_signed_with_payer(
        &[everlend_depositor::instruction::start_rebalancing(
            &everlend_depositor::id(),
            &Pubkey::new_unique(),
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
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(EverlendError::InvalidAccountOwner as u32),
        )
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
    ) = setup(100 * EXP).await;

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
    ) = setup(100 * EXP).await;

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
    ) = setup(100 * EXP).await;

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
    ) = setup(100 * EXP).await;

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
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(EverlendError::InvalidAccountOwner as u32)
        )
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
    ) = setup(100 * EXP).await;

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

#[tokio::test]
async fn rebalancing_math_round() {
    let mut d: DistributionArray = DistributionArray::default();
    let mut p = DistributionPubkeys::default();
    p[0] = Keypair::new().pubkey();
    p[1] = Keypair::new().pubkey();
    p[2] = Keypair::new().pubkey();

    let distr_amount: u64 = 4610400063;
    let mut oracle = TokenOracle::default();
    let mut r = Rebalancing::default();

    for (i, elem) in vec![
        (300_000_000, 300_000_000, 400_000_000),
        (300_000_000, 40_000_000, 660_000_000),
        (100_000_000, 100_000_000, 800_000_000),
        (0, 0, 1000_000_000),
        (300_000_000, 200_000_000, 500_000_000),
    ]
    .iter()
    .enumerate()
    {
        d[0] = elem.0;
        d[1] = elem.1;
        d[2] = elem.2;

        let current_slot = 1;
        oracle.reserve_rates.updated_at = current_slot;
        oracle
            .update_liquidity_distribution(i as u64 + 1, d)
            .unwrap();
        r.compute(&p, oracle.clone(), distr_amount, current_slot)
            .unwrap();
        println!("{}", r.total_distributed_liquidity().unwrap());
        assert_eq!(
            distr_amount >= r.total_distributed_liquidity().unwrap(),
            true
        );

        r.compute_with_refresh_income(&p, 0, i as u64 + 1, distr_amount)
            .unwrap();
        println!("{}", r.total_distributed_liquidity().unwrap());
        println!("{:?}", r.steps);
        assert_eq!(
            distr_amount >= r.total_distributed_liquidity().unwrap(),
            true
        );
    }
}

#[tokio::test]
async fn rebalancing_check_steps() {
    let mut d: DistributionArray = DistributionArray::default();
    let mut p = DistributionPubkeys::default();
    p[0] = Keypair::new().pubkey();
    p[1] = Keypair::new().pubkey();

    let distr_amount: u64 = 10001;
    let mut oracle = TokenOracle::default();
    let mut r = Rebalancing::default();

    struct TestCase {
        distribution: (u64, u64),
        steps: Vec<(u8, RebalancingOperation, u64, Option<u64>)>,
    }

    for (i, elem) in vec![
        TestCase {
            distribution: (500_000_000, 0),
            steps: vec![(0, RebalancingOperation::Deposit, 5000, None)],
        },
        TestCase {
            distribution: (500_000_000, 500_000_000),
            steps: vec![(1, RebalancingOperation::Deposit, 5000, None)],
        },
        TestCase {
            distribution: (490_000_000, 490_000_000),
            steps: vec![
                (0, RebalancingOperation::Withdraw, 100, Some(100)),
                (1, RebalancingOperation::Withdraw, 100, Some(100)),
            ],
        },
        TestCase {
            distribution: (1000_000_000, 0),
            steps: vec![
                (1, RebalancingOperation::Withdraw, 4900, Some(4900)),
                (0, RebalancingOperation::Deposit, 5101, None),
            ],
        },
        TestCase {
            distribution: (900_000_000, 100_000_000),
            steps: vec![
                (0, RebalancingOperation::Withdraw, 1001, Some(1001)),
                (1, RebalancingOperation::Deposit, 1000, None),
            ],
        },
    ]
    .iter()
    .enumerate()
    {
        d[0] = elem.distribution.0;
        d[1] = elem.distribution.1;

        let current_slot = 1;
        oracle.reserve_rates.updated_at = current_slot;
        oracle
            .update_liquidity_distribution(i as u64 + 1, d)
            .unwrap();
        r.compute(&p, oracle.clone(), distr_amount, current_slot)
            .unwrap();

        println!(
            "{} {}",
            r.amount_to_distribute,
            r.total_distributed_liquidity().unwrap()
        );

        for (idx, s) in r.clone().steps.iter().enumerate() {
            let mm_index = elem.steps[idx].0;
            let operation = elem.steps[idx].1;
            let liquidity_amount = elem.steps[idx].2;
            let collateral_amount = elem.steps[idx].3;

            assert_eq!(s.money_market_index, mm_index);
            assert_eq!(s.operation, operation);
            assert_eq!(s.liquidity_amount, liquidity_amount);
            assert_eq!(s.collateral_amount, collateral_amount);

            r.execute_step(s.operation, Some(liquidity_amount), (i + 2) as u64)
                .unwrap();
        }
    }
}

#[tokio::test]
async fn rebalancing_check_steps_math() {
    let mut p = DistributionPubkeys::default();
    p[0] = Keypair::new().pubkey();
    p[1] = Keypair::new().pubkey();
    p[2] = Keypair::new().pubkey();

    let mut d: DistributionArray = DistributionArray::default();
    d[0] = 500_000_000;
    d[1] = 500_000_000;

    let mut oracle = TokenOracle::default();
    oracle.liquidity_distribution.values = d;

    let mut received_collateral = [0; 10];
    received_collateral[0] = 5218140718;
    received_collateral[1] = 12821948839;

    let mut distributed_liquidity = [0; 10];
    distributed_liquidity[0] = 12821948839;
    distributed_liquidity[1] = 12821948839;

    let mut r = Rebalancing::default();
    r.amount_to_distribute = 25643897678;
    r.distributed_liquidity = distributed_liquidity;
    r.received_collateral = received_collateral;
    r.liquidity_distribution = oracle.liquidity_distribution.clone();

    d[0] = 333_333_333;
    d[1] = 333_333_333;
    d[2] = 333_333_333;

    oracle.update_liquidity_distribution(10, d).unwrap();

    let amount_to_distribute = 25365814993;
    let current_slot = 1;
    oracle.reserve_rates.updated_at = current_slot;
    r.compute(&p, oracle.clone(), amount_to_distribute, current_slot)
        .unwrap();

    println!("{:?}", r.steps);

    assert_eq!(r.steps[0].liquidity_amount, 4366677184);
    assert_eq!(r.steps[0].collateral_amount, Some(1777103957));
    assert_eq!(r.steps[1].liquidity_amount, 4366677184);
    assert_eq!(r.steps[1].collateral_amount, Some(4366677184));
    assert_eq!(r.steps[2].liquidity_amount, 8455271655);
    assert_eq!(r.steps[2].collateral_amount, None);
}

#[tokio::test]
async fn rebalancing_check_distribution_overflow() {
    let mut oracle = TokenOracle::default();
    let mut d: DistributionArray = DistributionArray::default();

    d[0] = 1;
    d[1] = u64::MAX;

    assert_eq!(
        oracle.update_liquidity_distribution(10, d).unwrap_err(),
        ProgramError::Custom(EverlendError::MathOverflow as u32)
    )
}

#[tokio::test]
async fn rebalancing_percent_ratio() {
    let prev_amount = 12821948839;
    let new_amount = 8455271655;

    let collateral_amount = prev_amount; // same as liquidity
    let amount = abs_diff(new_amount, prev_amount).unwrap();
    assert_eq!(amount, 4366677184);

    let collateral_amount = percent_ratio(amount, prev_amount, collateral_amount).unwrap();
    assert_eq!(collateral_amount, 4366677184);
}

#[tokio::test]
async fn collateral_leak_test() {
    let mut p = DistributionPubkeys::default();
    p[0] = Keypair::new().pubkey();
    p[1] = Keypair::new().pubkey();
    p[2] = Keypair::new().pubkey();

    let mut d: DistributionArray = DistributionArray::default();
    d[0] = 500_000_000;
    d[1] = 500_000_000;

    let mut oracle = TokenOracle::default();
    oracle.liquidity_distribution.values = d;

    let mut r = Rebalancing::default();

    let amount_to_distribute = 100_000_000;
    let current_slot = 1;

    oracle.liquidity_distribution.updated_at = current_slot;
    oracle.reserve_rates.updated_at = current_slot;
    r.compute(&p, oracle.clone(), amount_to_distribute, current_slot)
        .unwrap();

    assert_eq!(r.total_distributed_liquidity().unwrap(), 100_000_000);

    let mut reserve_rates: DistributionArray = DistributionArray::default();
    reserve_rates[0] = 1_000_000;

    d[0] = 500_000_100;
    d[1] = 499_999_900;

    let current_slot = 2;
    oracle
        .update_liquidity_distribution(current_slot, d)
        .unwrap();
    oracle
        .update_reserve_rates(current_slot, reserve_rates)
        .unwrap();

    r.compute(&p, oracle.clone(), amount_to_distribute, current_slot)
        .unwrap();

    assert_eq!(r.total_distributed_liquidity().unwrap(), 999_999_90);
}

#[tokio::test]
async fn collateral_leak_test2() {
    let mut d: DistributionArray = DistributionArray::default();
    let mut p = DistributionPubkeys::default();
    p[0] = Keypair::new().pubkey();
    p[1] = Keypair::new().pubkey();

    let mut oracle = TokenOracle::default();
    let mut rates = DistributionArray::default();
    let mut r = Rebalancing::default();

    struct TestCase {
        transit_amount: u64,
        distribution: (u64, u64),
        distributed_liquidity: u64,
        rates: (u64, u64),
        steps: Vec<(u8, RebalancingOperation, u64, Option<u64>)>,
    }

    let initial_liquidity = 595545109;

    for (i, elem) in vec![
        TestCase {
            transit_amount: initial_liquidity,
            distribution: (599999999, 400000001),
            rates: (1000000000, 1000000000),
            steps: vec![
                (0, RebalancingOperation::Deposit, 357327064, None),
                (1, RebalancingOperation::Deposit, 238218044, None),
            ],
            distributed_liquidity: 595545108,
        },
        TestCase {
            transit_amount: 1,
            distribution: (600000000, 400000000),
            rates: (1000000, 1000000000),
            steps: vec![(1, RebalancingOperation::Withdraw, 1, Some(1))],
            distributed_liquidity: 595545107,
        },
        TestCase {
            transit_amount: 2,
            distribution: (600000000, 400000000),
            rates: (1000000, 1000000),
            steps: vec![],
            distributed_liquidity: 595545107,
        },
    ]
    .iter()
    .enumerate()
    {
        d[0] = elem.distribution.0;
        d[1] = elem.distribution.1;

        rates[0] = elem.rates.0;
        rates[1] = elem.rates.1;

        let current_slot = 1;
        oracle.update_reserve_rates(current_slot, rates).unwrap();
        oracle
            .update_liquidity_distribution(i as u64 + 1, d)
            .unwrap();

        let (_, amount_to_distribute) = calculate_amount_to_distribute(
            r.total_distributed_liquidity().unwrap(),
            elem.transit_amount,
            0,
            0,
        )
        .unwrap();

        r.compute(&p, oracle.clone(), amount_to_distribute, current_slot)
            .unwrap();

        println!(
            "amount_to_distribute: {} distributed_liquidity:{} \n\n",
            r.amount_to_distribute,
            r.total_distributed_liquidity().unwrap()
        );

        assert_eq!(
            r.total_distributed_liquidity().unwrap(),
            elem.distributed_liquidity
        );

        for (idx, s) in r.clone().steps.iter().enumerate() {
            let mm_index = elem.steps[idx].0;
            let operation = elem.steps[idx].1;
            let liquidity_amount = elem.steps[idx].2;
            let collateral_amount = elem.steps[idx].3;

            assert_eq!(s.money_market_index, mm_index);
            assert_eq!(s.operation, operation);
            assert_eq!(s.liquidity_amount, liquidity_amount);
            assert_eq!(s.collateral_amount, collateral_amount);

            r.execute_step(s.operation, Some(liquidity_amount), (i + 2) as u64)
                .unwrap();
        }
    }
}
