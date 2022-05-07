#![cfg(feature = "test-bpf")]

use everlend_registry::state::{RegistryRootAccounts, DistributionPubkeys};
use solana_program::instruction::InstructionError;
use solana_program::{program_pack::Pack, pubkey::Pubkey};
use solana_program_test::*;
use solana_sdk::signer::Signer;
use solana_sdk::transaction::{Transaction, TransactionError};
use spl_token_lending::error::LendingError;

use everlend_liquidity_oracle::state::DistributionArray;
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
    TestPoolMarket,
    TestPool,
    LiquidityProvider,
    TestDepositor,
    TestLiquidityOracle,
    TestTokenDistribution,
    DistributionArray,
) {
    let (mut context, money_market, pyth_oracle, registry) = presetup().await;

    let payer_pubkey = context.payer.pubkey();

    // 0. Prepare lending
    let reserve = money_market.get_reserve_data(&mut context).await;
    // println!("{:#?}", reserve);

    let account = get_account(&mut context, &money_market.market_pubkey).await;
    let lending_market =
        spl_token_lending::state::LendingMarket::unpack_from_slice(account.data.as_slice())
            .unwrap();

    let authority_signer_seeds = &[
        &money_market.market_pubkey.to_bytes()[..32],
        &[lending_market.bump_seed],
    ];
    let lending_market_authority_pubkey =
        Pubkey::create_program_address(authority_signer_seeds, &spl_token_lending::id()).unwrap();

    println!("{:#?}", lending_market_authority_pubkey);

    let collateral_mint = get_mint_data(&mut context, &reserve.collateral.mint_pubkey).await;
    println!("{:#?}", collateral_mint);

    // 1. Prepare general pool

    let general_pool_market = TestGeneralPoolMarket::new();
    general_pool_market.init(&mut context).await.unwrap();

    let general_pool = TestGeneralPool::new(&general_pool_market, None);
    general_pool
        .create(&mut context, &general_pool_market)
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

    general_pool
        .deposit(
            &mut context,
            &general_pool_market,
            &liquidity_provider,
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

    // 4.3 Create transit account for collateral token
    test_depositor
        .create_transit(&mut context, &mm_pool.token_mint_pubkey, None)
        .await
        .unwrap();

    // 34.4 Create transit account for mm pool collateral token
    test_depositor
        .create_transit(&mut context, &mm_pool.pool_mint.pubkey(), None)
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
            ULP_SHARE_ALLOWED,
        )
        .await
        .unwrap();

    let mut roots = RegistryRootAccounts {
        general_pool_market: general_pool_market.keypair.pubkey(),
        income_pool_market: income_pool_market.keypair.pubkey(),
        collateral_pool_markets: DistributionPubkeys::default(),
        liquidity_oracle: test_liquidity_oracle.keypair.pubkey(),
    };
    roots.collateral_pool_markets[0] = mm_pool_market.keypair.pubkey();
    registry
        .set_registry_root_accounts(&mut context, roots)
        .await
        .unwrap();

    // 6. Start rebalancing
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

    (
        context,
        money_market,
        pyth_oracle,
        registry,
        general_pool_market,
        general_pool,
        general_pool_borrow_authority,
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
        money_market,
        pyth_oracle,
        registry,
        _general_pool_market,
        general_pool,
        _general_pool_borrow_authority,
        mm_pool_market,
        mm_pool,
        _liquidity_provider,
        test_depositor,
        _,
        _,
        _,
    ) = setup().await;

    let reserve = money_market.get_reserve_data(&mut context).await;
    let reserve_balance_before =
        get_token_balance(&mut context, &reserve.liquidity.supply_pubkey).await;

    // Rates should be refreshed
    context.warp_to_slot(3).unwrap();
    pyth_oracle.update(&mut context, 3).await;

    let rebalancing = test_depositor
        .get_rebalancing_data(&mut context, &general_pool.token_mint_pubkey)
        .await;

    println!("rebalancing = {:#?}", rebalancing);

    let money_market_pubkeys =
        MoneyMarketPubkeys::SPL(integrations::spl_token_lending::AccountPubkeys {
            reserve: money_market.reserve_pubkey,
            reserve_liquidity_supply: reserve.liquidity.supply_pubkey,
            reserve_liquidity_oracle: reserve.liquidity.oracle_pubkey,
            lending_market: money_market.market_pubkey,
        });

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

    assert_eq!(
        get_token_balance(&mut context, &mm_pool.token_account.pubkey()).await,
        rebalancing.steps[0].liquidity_amount,
    );
    assert_eq!(
        get_token_balance(&mut context, &reserve.liquidity.supply_pubkey).await,
        reserve_balance_before + rebalancing.steps[0].liquidity_amount,
    );

    let (liquidity_transit, _) = everlend_depositor::find_transit_program_address(
        &everlend_depositor::id(),
        &test_depositor.depositor.pubkey(),
        &general_pool.token_mint_pubkey,
        "",
    );
    assert_eq!(
        get_token_balance(&mut context, &liquidity_transit).await,
        100 * EXP - rebalancing.steps[0].liquidity_amount,
    );
}

#[tokio::test]
async fn success_increased_liquidity() {
    let (
        mut context,
        money_market,
        pyth_oracle,
        registry,
        general_pool_market,
        general_pool,
        _general_pool_borrow_authority,
        mm_pool_market,
        mm_pool,
        liquidity_provider,
        test_depositor,
        test_liquidity_oracle,
        test_token_distribution,
        distribution,
    ) = setup().await;
    let payer_pubkey = context.payer.pubkey();

    // Rates should be refreshed
    context.warp_to_slot(3).unwrap();
    pyth_oracle.update(&mut context, 3).await;

    let reserve = money_market.get_reserve_data(&mut context).await;
    let money_market_pubkeys =
        MoneyMarketPubkeys::SPL(integrations::spl_token_lending::AccountPubkeys {
            reserve: money_market.reserve_pubkey,
            reserve_liquidity_supply: reserve.liquidity.supply_pubkey,
            reserve_liquidity_oracle: reserve.liquidity.oracle_pubkey,
            lending_market: money_market.market_pubkey,
        });

    // 1. Complete first rebalancing

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

    // 2. Update token distribution

    test_token_distribution
        .update(
            &mut context,
            &test_liquidity_oracle,
            payer_pubkey,
            distribution,
        )
        .await
        .unwrap();

    // 3. Add liquidity
    general_pool
        .deposit(
            &mut context,
            &general_pool_market,
            &liquidity_provider,
            50 * EXP,
        )
        .await
        .unwrap();

    // 4. Start new rebalancing with new liquidity

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

    // let rebalancing = test_depositor
    //     .get_rebalancing_data(&mut context, &general_pool.token_mint_pubkey)
    //     .await;

    // println!("Rebalancing: {:#?}", rebalancing);
}

#[tokio::test]
async fn fail_with_invalid_registry() {
    let (
        mut context,
        money_market,
        pyth_oracle,
        _,
        _,
        _,
        _,
        mm_pool_market,
        mm_pool,
        _,
        test_depositor,
        _,
        _,
        _,
    ) = setup().await;

    let reserve = money_market.get_reserve_data(&mut context).await;

    // Rates should be refreshed
    context.warp_to_slot(3).unwrap();
    pyth_oracle.update(&mut context, 3).await;

    let money_market_pubkeys =
        MoneyMarketPubkeys::SPL(integrations::spl_token_lending::AccountPubkeys {
            reserve: money_market.reserve_pubkey,
            reserve_liquidity_supply: reserve.liquidity.supply_pubkey,
            reserve_liquidity_oracle: reserve.liquidity.oracle_pubkey,
            lending_market: money_market.market_pubkey,
        });

    let deposit_accounts =
        integrations::deposit_accounts(&spl_token_lending::id(), &money_market_pubkeys);

    let tx = Transaction::new_signed_with_payer(
        &[everlend_depositor::instruction::deposit(
            &everlend_depositor::id(),
            &Pubkey::new_unique(),
            &test_depositor.depositor.pubkey(),
            &mm_pool_market.keypair.pubkey(),
            &mm_pool.token_account.pubkey(),
            &mm_pool.pool_mint.pubkey(),
            &get_liquidity_mint().1,
            &mm_pool.token_mint_pubkey,
            &spl_token_lending::id(),
            deposit_accounts,
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
        money_market,
        pyth_oracle,
        registry,
        _,
        _,
        _,
        mm_pool_market,
        mm_pool,
        _,
        _,
        _,
        _,
        _,
    ) = setup().await;

    let reserve = money_market.get_reserve_data(&mut context).await;

    // Rates should be refreshed
    context.warp_to_slot(3).unwrap();
    pyth_oracle.update(&mut context, 3).await;

    let money_market_pubkeys =
        MoneyMarketPubkeys::SPL(integrations::spl_token_lending::AccountPubkeys {
            reserve: money_market.reserve_pubkey,
            reserve_liquidity_supply: reserve.liquidity.supply_pubkey,
            reserve_liquidity_oracle: reserve.liquidity.oracle_pubkey,
            lending_market: money_market.market_pubkey,
        });

    let deposit_accounts =
        integrations::deposit_accounts(&spl_token_lending::id(), &money_market_pubkeys);

    let tx = Transaction::new_signed_with_payer(
        &[everlend_depositor::instruction::deposit(
            &everlend_depositor::id(),
            &registry.keypair.pubkey(),
            &Pubkey::new_unique(),
            &mm_pool_market.keypair.pubkey(),
            &mm_pool.token_account.pubkey(),
            &mm_pool.pool_mint.pubkey(),
            &get_liquidity_mint().1,
            &mm_pool.token_mint_pubkey,
            &spl_token_lending::id(),
            deposit_accounts,
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
async fn fail_with_invalid_mm_pool_market() {
    let (
        mut context,
        money_market,
        pyth_oracle,
        registry,
        _,
        _,
        _,
        _,
        mm_pool,
        _,
        test_depositor,
        _,
        _,
        _,
    ) = setup().await;

    let reserve = money_market.get_reserve_data(&mut context).await;

    // Rates should be refreshed
    context.warp_to_slot(3).unwrap();
    pyth_oracle.update(&mut context, 3).await;

    let money_market_pubkeys =
        MoneyMarketPubkeys::SPL(integrations::spl_token_lending::AccountPubkeys {
            reserve: money_market.reserve_pubkey,
            reserve_liquidity_supply: reserve.liquidity.supply_pubkey,
            reserve_liquidity_oracle: reserve.liquidity.oracle_pubkey,
            lending_market: money_market.market_pubkey,
        });

    let deposit_accounts =
        integrations::deposit_accounts(&spl_token_lending::id(), &money_market_pubkeys);

    let tx = Transaction::new_signed_with_payer(
        &[everlend_depositor::instruction::deposit(
            &everlend_depositor::id(),
            &registry.keypair.pubkey(),
            &test_depositor.depositor.pubkey(),
            &Pubkey::new_unique(),
            &mm_pool.token_account.pubkey(),
            &mm_pool.pool_mint.pubkey(),
            &get_liquidity_mint().1,
            &mm_pool.token_mint_pubkey,
            &spl_token_lending::id(),
            deposit_accounts,
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
async fn fail_with_invalid_mm_pool_token_account() {
    let (
        mut context,
        money_market,
        pyth_oracle,
        registry,
        _,
        _,
        _,
        mm_pool_market,
        mm_pool,
        _,
        test_depositor,
        _,
        _,
        _,
    ) = setup().await;

    let reserve = money_market.get_reserve_data(&mut context).await;

    // Rates should be refreshed
    context.warp_to_slot(3).unwrap();
    pyth_oracle.update(&mut context, 3).await;

    let money_market_pubkeys =
        MoneyMarketPubkeys::SPL(integrations::spl_token_lending::AccountPubkeys {
            reserve: money_market.reserve_pubkey,
            reserve_liquidity_supply: reserve.liquidity.supply_pubkey,
            reserve_liquidity_oracle: reserve.liquidity.oracle_pubkey,
            lending_market: money_market.market_pubkey,
        });

    let deposit_accounts =
        integrations::deposit_accounts(&spl_token_lending::id(), &money_market_pubkeys);

    let tx = Transaction::new_signed_with_payer(
        &[everlend_depositor::instruction::deposit(
            &everlend_depositor::id(),
            &registry.keypair.pubkey(),
            &test_depositor.depositor.pubkey(),
            &mm_pool_market.keypair.pubkey(),
            &Pubkey::new_unique(),
            &mm_pool.pool_mint.pubkey(),
            &get_liquidity_mint().1,
            &mm_pool.token_mint_pubkey,
            &spl_token_lending::id(),
            deposit_accounts,
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
async fn fail_with_invalid_mm_pool_collateral_mint() {
    let (
        mut context,
        money_market,
        pyth_oracle,
        registry,
        _,
        _,
        _,
        mm_pool_market,
        mm_pool,
        _,
        test_depositor,
        _,
        _,
        _,
    ) = setup().await;

    let reserve = money_market.get_reserve_data(&mut context).await;

    // Rates should be refreshed
    context.warp_to_slot(3).unwrap();
    pyth_oracle.update(&mut context, 3).await;

    let money_market_pubkeys =
        MoneyMarketPubkeys::SPL(integrations::spl_token_lending::AccountPubkeys {
            reserve: money_market.reserve_pubkey,
            reserve_liquidity_supply: reserve.liquidity.supply_pubkey,
            reserve_liquidity_oracle: reserve.liquidity.oracle_pubkey,
            lending_market: money_market.market_pubkey,
        });

    let deposit_accounts =
        integrations::deposit_accounts(&spl_token_lending::id(), &money_market_pubkeys);

    let tx = Transaction::new_signed_with_payer(
        &[everlend_depositor::instruction::deposit(
            &everlend_depositor::id(),
            &registry.keypair.pubkey(),
            &test_depositor.depositor.pubkey(),
            &mm_pool_market.keypair.pubkey(),
            &mm_pool.token_account.pubkey(),
            &Pubkey::new_unique(),
            &get_liquidity_mint().1,
            &mm_pool.token_mint_pubkey,
            &spl_token_lending::id(),
            deposit_accounts,
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
async fn fail_with_invalid_liquidity_mint() {
    let (
        mut context,
        money_market,
        pyth_oracle,
        registry,
        _,
        _,
        _,
        mm_pool_market,
        mm_pool,
        _,
        test_depositor,
        _,
        _,
        _,
    ) = setup().await;

    let reserve = money_market.get_reserve_data(&mut context).await;

    // Rates should be refreshed
    context.warp_to_slot(3).unwrap();
    pyth_oracle.update(&mut context, 3).await;

    let money_market_pubkeys =
        MoneyMarketPubkeys::SPL(integrations::spl_token_lending::AccountPubkeys {
            reserve: money_market.reserve_pubkey,
            reserve_liquidity_supply: reserve.liquidity.supply_pubkey,
            reserve_liquidity_oracle: reserve.liquidity.oracle_pubkey,
            lending_market: money_market.market_pubkey,
        });

    let deposit_accounts =
        integrations::deposit_accounts(&spl_token_lending::id(), &money_market_pubkeys);

    let tx = Transaction::new_signed_with_payer(
        &[everlend_depositor::instruction::deposit(
            &everlend_depositor::id(),
            &registry.keypair.pubkey(),
            &test_depositor.depositor.pubkey(),
            &mm_pool_market.keypair.pubkey(),
            &mm_pool.token_account.pubkey(),
            &mm_pool.pool_mint.pubkey(),
            &Pubkey::new_unique(),
            &mm_pool.token_mint_pubkey,
            &spl_token_lending::id(),
            deposit_accounts,
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
async fn fail_with_invalid_collateral_mint() {
    let (
        mut context,
        money_market,
        pyth_oracle,
        registry,
        _,
        _,
        _,
        mm_pool_market,
        mm_pool,
        _,
        test_depositor,
        _,
        _,
        _,
    ) = setup().await;

    let reserve = money_market.get_reserve_data(&mut context).await;

    // Rates should be refreshed
    context.warp_to_slot(3).unwrap();
    pyth_oracle.update(&mut context, 3).await;

    let money_market_pubkeys =
        MoneyMarketPubkeys::SPL(integrations::spl_token_lending::AccountPubkeys {
            reserve: money_market.reserve_pubkey,
            reserve_liquidity_supply: reserve.liquidity.supply_pubkey,
            reserve_liquidity_oracle: reserve.liquidity.oracle_pubkey,
            lending_market: money_market.market_pubkey,
        });

    let deposit_accounts =
        integrations::deposit_accounts(&spl_token_lending::id(), &money_market_pubkeys);

    let tx = Transaction::new_signed_with_payer(
        &[everlend_depositor::instruction::deposit(
            &everlend_depositor::id(),
            &registry.keypair.pubkey(),
            &test_depositor.depositor.pubkey(),
            &mm_pool_market.keypair.pubkey(),
            &mm_pool.token_account.pubkey(),
            &mm_pool.pool_mint.pubkey(),
            &get_liquidity_mint().1,
            &Pubkey::new_unique(),
            &spl_token_lending::id(),
            deposit_accounts,
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
            InstructionError::Custom(LendingError::InvalidAccountInput as u32),
        )
    );
}

#[tokio::test]
async fn fail_with_invalid_money_market_program_id() {
    let (
        mut context,
        money_market,
        pyth_oracle,
        registry,
        _,
        _,
        _,
        mm_pool_market,
        mm_pool,
        _,
        test_depositor,
        _,
        _,
        _,
    ) = setup().await;

    let reserve = money_market.get_reserve_data(&mut context).await;

    // Rates should be refreshed
    context.warp_to_slot(3).unwrap();
    pyth_oracle.update(&mut context, 3).await;

    let money_market_pubkeys =
        MoneyMarketPubkeys::SPL(integrations::spl_token_lending::AccountPubkeys {
            reserve: money_market.reserve_pubkey,
            reserve_liquidity_supply: reserve.liquidity.supply_pubkey,
            reserve_liquidity_oracle: reserve.liquidity.oracle_pubkey,
            lending_market: money_market.market_pubkey,
        });

    let deposit_accounts =
        integrations::deposit_accounts(&spl_token_lending::id(), &money_market_pubkeys);

    let tx = Transaction::new_signed_with_payer(
        &[everlend_depositor::instruction::deposit(
            &everlend_depositor::id(),
            &registry.keypair.pubkey(),
            &test_depositor.depositor.pubkey(),
            &mm_pool_market.keypair.pubkey(),
            &mm_pool.token_account.pubkey(),
            &mm_pool.pool_mint.pubkey(),
            &get_liquidity_mint().1,
            &mm_pool.token_mint_pubkey,
            &Pubkey::new_unique(),
            deposit_accounts,
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
            InstructionError::Custom(EverlendError::InvalidRebalancingMoneyMarket as u32),
        )
    );
}

#[tokio::test]
async fn fail_with_invalid_money_market_accounts() {
    let (
        mut context,
        _,
        pyth_oracle,
        registry,
        _,
        _,
        _,
        mm_pool_market,
        mm_pool,
        _,
        test_depositor,
        _,
        _,
        _,
    ) = setup().await;

    // Rates should be refreshed
    context.warp_to_slot(3).unwrap();
    pyth_oracle.update(&mut context, 3).await;

    let deposit_accounts = vec![];

    let tx = Transaction::new_signed_with_payer(
        &[everlend_depositor::instruction::deposit(
            &everlend_depositor::id(),
            &registry.keypair.pubkey(),
            &test_depositor.depositor.pubkey(),
            &mm_pool_market.keypair.pubkey(),
            &mm_pool.token_account.pubkey(),
            &mm_pool.pool_mint.pubkey(),
            &get_liquidity_mint().1,
            &mm_pool.token_mint_pubkey,
            &spl_token_lending::id(),
            deposit_accounts,
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
        TransactionError::InstructionError(0, InstructionError::NotEnoughAccountKeys)
    );
}
