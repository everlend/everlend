use everlend_registry::instructions::{UpdateRegistryData, UpdateRegistryMarketsData};
use solana_program::instruction::InstructionError;
use solana_program::{program_pack::Pack, pubkey::Pubkey};
use solana_program_test::*;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use solana_sdk::transaction::{Transaction, TransactionError};

use everlend_liquidity_oracle::state::DistributionArray;
use everlend_registry::state::DistributionPubkeys;
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
    Pubkey,
) {
    let mut env = presetup().await;

    let payer_pubkey = env.context.payer.pubkey();

    // 1. Prepare lending
    let reserve = env
        .spl_token_lending
        .get_reserve_data(&mut env.context)
        .await;

    let account = get_account(&mut env.context, &env.spl_token_lending.market_pubkey).await;
    let lending_market =
        spl_token_lending::state::LendingMarket::unpack_from_slice(account.data.as_slice())
            .unwrap();

    let authority_signer_seeds = &[
        &env.spl_token_lending.market_pubkey.to_bytes()[..32],
        &[lending_market.bump_seed],
    ];
    let _lending_market_authority_pubkey =
        Pubkey::create_program_address(authority_signer_seeds, &spl_token_lending::id()).unwrap();

    get_mint_data(&mut env.context, &reserve.collateral.mint_pubkey).await;

    // 2.1 Prepare general pool

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

    // 2.2 Add liquidity to general pool

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
            100 * EXP,
        )
        .await
        .unwrap();

    // 3. Prepare income pool
    let income_pool_market = TestIncomePoolMarket::new();
    income_pool_market
        .init(&mut env.context, &general_pool_market)
        .await
        .unwrap();

    // 4. Prepare money market pool

    let mm_pool_market = TestPoolMarket::new();
    mm_pool_market.init(&mut env.context).await.unwrap();

    let mm_pool = TestPool::new(&mm_pool_market, Some(reserve.collateral.mint_pubkey));
    mm_pool
        .create(&mut env.context, &mm_pool_market)
        .await
        .unwrap();

    // 5. Prepare depositor

    // 5.1. Prepare liquidity oracle

    let test_liquidity_oracle = TestLiquidityOracle::new();
    test_liquidity_oracle.init(&mut env.context).await.unwrap();

    let mut distribution = DistributionArray::default();
    distribution[0] = 500_000_000u64; // 50%

    let test_token_distribution =
        TestTokenDistribution::new(general_pool.token_mint_pubkey, distribution);

    test_token_distribution
        .init(&mut env.context, &test_liquidity_oracle, payer_pubkey)
        .await
        .unwrap();

    test_token_distribution
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

    // 5.2 Create transit account for liquidity token
    test_depositor
        .create_transit(&mut env.context, &general_pool.token_mint_pubkey, None)
        .await
        .unwrap();

    // 5.3 Create transit account for collateral token
    test_depositor
        .create_transit(&mut env.context, &mm_pool.token_mint_pubkey, None)
        .await
        .unwrap();

    // 6. Prepare borrow authority
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

    let mut collateral_pool_markets = DistributionPubkeys::default();
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

    // 7. Start rebalancing
    test_depositor
        .start_rebalancing(
            &mut env.context,
            &env.registry,
            &general_pool_market,
            &general_pool,
            &test_liquidity_oracle,
            false,
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
        mm_pool_market,
        mm_pool,
        liquidity_provider,
        test_depositor,
        test_liquidity_oracle,
        test_token_distribution,
        distribution,
        mining_acc,
    )
}

#[tokio::test]
async fn success() {
    let (
        mut context,
        spl_token_lending,
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
        _,
    ) = setup().await;

    let reserve = spl_token_lending.get_reserve_data(&mut context).await;
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
            reserve: spl_token_lending.reserve_pubkey,
            reserve_liquidity_supply: reserve.liquidity.supply_pubkey,
            reserve_liquidity_oracle: reserve.liquidity.oracle_pubkey,
            lending_market: spl_token_lending.market_pubkey,
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
        mining_acc,
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
            mining_acc,
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
    let deposit_collateral_storage_accounts = mm_pool.deposit_accounts(&mm_pool_market);

    let tx = Transaction::new_signed_with_payer(
        &[everlend_depositor::instruction::deposit(
            &everlend_depositor::id(),
            &Pubkey::new_unique(),
            &test_depositor.depositor.pubkey(),
            &get_liquidity_mint().1,
            &mm_pool.token_mint_pubkey,
            &context.payer.pubkey(),
            &spl_token_lending::id(),
            deposit_accounts,
            deposit_collateral_storage_accounts,
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

    let deposit_collateral_storage_accounts = mm_pool.deposit_accounts(&mm_pool_market);
    let tx = Transaction::new_signed_with_payer(
        &[everlend_depositor::instruction::deposit(
            &everlend_depositor::id(),
            &registry.keypair.pubkey(),
            &Pubkey::new_unique(),
            &get_liquidity_mint().1,
            &mm_pool.token_mint_pubkey,
            &context.payer.pubkey(),
            &spl_token_lending::id(),
            deposit_accounts,
            deposit_collateral_storage_accounts,
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

    let deposit_collateral_storage_accounts = mm_pool.deposit_accounts(&TestPoolMarket {
        keypair: Keypair::new(),
        manager: Keypair::new(),
    });

    let tx = Transaction::new_signed_with_payer(
        &[everlend_depositor::instruction::deposit(
            &everlend_depositor::id(),
            &registry.keypair.pubkey(),
            &test_depositor.depositor.pubkey(),
            &get_liquidity_mint().1,
            &mm_pool.token_mint_pubkey,
            &context.payer.pubkey(),
            &spl_token_lending::id(),
            deposit_accounts,
            deposit_collateral_storage_accounts,
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

    let mock_mm_pool = TestPool {
        pool_pubkey: mm_pool.pool_pubkey,
        token_mint_pubkey: mm_pool.token_mint_pubkey,
        token_account: Keypair::new(),
    };
    let deposit_collateral_storage_accounts = mock_mm_pool.deposit_accounts(&mm_pool_market);

    let tx = Transaction::new_signed_with_payer(
        &[everlend_depositor::instruction::deposit(
            &everlend_depositor::id(),
            &registry.keypair.pubkey(),
            &test_depositor.depositor.pubkey(),
            &get_liquidity_mint().1,
            &mm_pool.token_mint_pubkey,
            &context.payer.pubkey(),
            &spl_token_lending::id(),
            deposit_accounts,
            deposit_collateral_storage_accounts,
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

    let deposit_collateral_storage_accounts = mm_pool.deposit_accounts(&mm_pool_market);

    let tx = Transaction::new_signed_with_payer(
        &[everlend_depositor::instruction::deposit(
            &everlend_depositor::id(),
            &registry.keypair.pubkey(),
            &test_depositor.depositor.pubkey(),
            &Pubkey::new_unique(),
            &mm_pool.token_mint_pubkey,
            &context.payer.pubkey(),
            &spl_token_lending::id(),
            deposit_accounts,
            deposit_collateral_storage_accounts,
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

    let collateral_mint = mm_pool.token_mint_pubkey.clone();

    let mock_mm_pool = TestPool {
        pool_pubkey: mm_pool.pool_pubkey,
        token_mint_pubkey: Pubkey::new_unique(),
        token_account: mm_pool.token_account,
    };
    let deposit_collateral_storage_accounts = mock_mm_pool.deposit_accounts(&mm_pool_market);

    let tx = Transaction::new_signed_with_payer(
        &[everlend_depositor::instruction::deposit(
            &everlend_depositor::id(),
            &registry.keypair.pubkey(),
            &test_depositor.depositor.pubkey(),
            &get_liquidity_mint().1,
            &collateral_mint,
            &context.payer.pubkey(),
            &spl_token_lending::id(),
            deposit_accounts,
            deposit_collateral_storage_accounts,
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

    let deposit_collateral_storage_accounts = mm_pool.deposit_accounts(&mm_pool_market);

    let tx = Transaction::new_signed_with_payer(
        &[everlend_depositor::instruction::deposit(
            &everlend_depositor::id(),
            &registry.keypair.pubkey(),
            &test_depositor.depositor.pubkey(),
            &get_liquidity_mint().1,
            &mm_pool.token_mint_pubkey,
            &context.payer.pubkey(),
            &Pubkey::new_unique(),
            deposit_accounts,
            deposit_collateral_storage_accounts,
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
            InstructionError::Custom(EverlendError::IncorrectInstructionProgramId as u32),
        )
    );
}

#[tokio::test]
async fn fail_with_invalid_money_market_accounts() {
    let (mut context, _, pyth_oracle, registry, _, _, _, _, mm_pool, _, test_depositor, _, _, _, _) =
        setup().await;

    // Rates should be refreshed
    context.warp_to_slot(3).unwrap();
    pyth_oracle.update(&mut context, 3).await;

    let deposit_accounts = vec![];
    let deposit_collateral_storage_accounts = vec![];

    let tx = Transaction::new_signed_with_payer(
        &[everlend_depositor::instruction::deposit(
            &everlend_depositor::id(),
            &registry.keypair.pubkey(),
            &test_depositor.depositor.pubkey(),
            &get_liquidity_mint().1,
            &mm_pool.token_mint_pubkey,
            &context.payer.pubkey(),
            &spl_token_lending::id(),
            deposit_accounts,
            deposit_collateral_storage_accounts,
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
