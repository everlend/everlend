#![cfg(feature = "test-bpf")]

use solana_program::instruction::InstructionError;
use solana_program::{program_pack::Pack, pubkey::Pubkey};
use solana_program_test::*;
use solana_sdk::signer::Signer;
use solana_sdk::transaction::{Transaction, TransactionError};

use everlend_depositor::find_transit_program_address;
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
    TestIncomePoolMarket,
    TestIncomePool,
    TestPoolMarket,
    TestPool,
    LiquidityProvider,
    TestDepositor,
) {
    let (mut context, money_market, pyth_oracle, registry) = presetup().await;

    let payer_pubkey = context.payer.pubkey();

    // 0. Prepare lending
    let reserve = money_market.get_reserve_data(&mut context).await;
    println!("{:#?}", reserve);

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

    // 4.4 Create transit account for mm pool collateral token
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

    // 7. Deposit

    // Rates should be refreshed
    context.warp_to_slot(3).unwrap();
    pyth_oracle.update(&mut context, 3).await;
    // money_market.refresh_reserve(&mut context, 3).await;

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

    // 7.1 Decrease distribution & restart rebalancing

    distribution[0] = 0u64; // Decrease to 0%
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
        income_pool_market,
        income_pool,
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
        money_market,
        pyth_oracle,
        registry,
        _general_pool_market,
        general_pool,
        _general_pool_borrow_authority,
        income_pool_market,
        income_pool,
        mm_pool_market,
        mm_pool,
        _,
        test_depositor,
    ) = setup().await;

    let reserve = money_market.get_reserve_data(&mut context).await;
    let reserve_balance_before =
        get_token_balance(&mut context, &reserve.liquidity.supply_pubkey).await;
    let money_market_pubkeys =
        MoneyMarketPubkeys::SPL(integrations::spl_token_lending::AccountPubkeys {
            reserve: money_market.reserve_pubkey,
            reserve_liquidity_supply: reserve.liquidity.supply_pubkey,
            reserve_liquidity_oracle: reserve.liquidity.oracle_pubkey,
            lending_market: money_market.market_pubkey,
        });

    context.warp_to_slot(5).unwrap();
    pyth_oracle.update(&mut context, 5).await;

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

    let rebalancing = test_depositor
        .get_rebalancing_data(&mut context, &general_pool.token_mint_pubkey)
        .await;

    println!("rebalancing = {:#?}", rebalancing);

    assert!(rebalancing.is_completed());
    assert_eq!(
        get_token_balance(&mut context, &mm_pool.token_account.pubkey()).await,
        rebalancing.received_collateral[0],
    );
    assert_eq!(
        get_token_balance(&mut context, &reserve.liquidity.supply_pubkey).await,
        reserve_balance_before - rebalancing.steps[0].liquidity_amount,
    );
}

#[tokio::test]
async fn success_with_incomes() {
    let (
        mut context,
        money_market,
        pyth_oracle,
        registry,
        _general_pool_market,
        general_pool,
        _general_pool_borrow_authority,
        income_pool_market,
        income_pool,
        mm_pool_market,
        mm_pool,
        liquidity_provider,
        test_depositor,
    ) = setup().await;

    let mut reserve = money_market.get_reserve_data(&mut context).await;
    let money_market_pubkeys =
        MoneyMarketPubkeys::SPL(integrations::spl_token_lending::AccountPubkeys {
            reserve: money_market.reserve_pubkey,
            reserve_liquidity_supply: reserve.liquidity.supply_pubkey,
            reserve_liquidity_oracle: reserve.liquidity.oracle_pubkey,
            lending_market: money_market.market_pubkey,
        });

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

    let reserve_balance_before =
        get_token_balance(&mut context, &reserve.liquidity.supply_pubkey).await;

    context.warp_to_slot(5).unwrap();
    pyth_oracle.update(&mut context, 5).await;

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

    let income_balance = get_token_balance(&mut context, &income_pool.token_account.pubkey()).await;
    println!("Income balance: {}", income_balance);
    assert!(income_balance > 0);

    let rebalancing = test_depositor
        .get_rebalancing_data(&mut context, &general_pool.token_mint_pubkey)
        .await;

    assert!(rebalancing.is_completed());
    assert_eq!(
        get_token_balance(&mut context, &mm_pool.token_account.pubkey()).await,
        rebalancing.received_collateral[0],
    );
    assert_eq!(
        get_token_balance(&mut context, &reserve.liquidity.supply_pubkey).await,
        reserve_balance_before - rebalancing.steps[0].liquidity_amount - income_balance,
    );
}

#[tokio::test]
async fn fail_with_invalid_registry() {
    let (
        mut context,
        money_market,
        pyth_oracle,
        _,
        _general_pool_market,
        _,
        _general_pool_borrow_authority,
        income_pool_market,
        income_pool,
        mm_pool_market,
        mm_pool,
        _,
        test_depositor,
    ) = setup().await;

    let reserve = money_market.get_reserve_data(&mut context).await;

    let money_market_pubkeys =
        MoneyMarketPubkeys::SPL(integrations::spl_token_lending::AccountPubkeys {
            reserve: money_market.reserve_pubkey,
            reserve_liquidity_supply: reserve.liquidity.supply_pubkey,
            reserve_liquidity_oracle: reserve.liquidity.oracle_pubkey,
            lending_market: money_market.market_pubkey,
        });

    context.warp_to_slot(5).unwrap();
    pyth_oracle.update(&mut context, 5).await;

    let money_market_program_id = &spl_token_lending::id();

    let collateral_mint = mm_pool.token_mint_pubkey;
    let liquidity_mint = get_liquidity_mint().1;
    let mm_pool_collateral_mint = mm_pool.pool_mint.pubkey();

    let withdraw_accounts =
        integrations::withdraw_accounts(money_market_program_id, &money_market_pubkeys);

    let tx = Transaction::new_signed_with_payer(
        &[everlend_depositor::instruction::withdraw(
            &everlend_depositor::id(),
            &Pubkey::new_unique(),
            &test_depositor.depositor.pubkey(),
            &income_pool_market.keypair.pubkey(),
            &income_pool.token_account.pubkey(),
            &mm_pool_market.keypair.pubkey(),
            &mm_pool.token_account.pubkey(),
            &mm_pool_collateral_mint,
            &collateral_mint,
            &liquidity_mint,
            money_market_program_id,
            withdraw_accounts,
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
        _general_pool_market,
        _,
        _general_pool_borrow_authority,
        income_pool_market,
        income_pool,
        mm_pool_market,
        mm_pool,
        _,
        _,
    ) = setup().await;

    let reserve = money_market.get_reserve_data(&mut context).await;

    let money_market_pubkeys =
        MoneyMarketPubkeys::SPL(integrations::spl_token_lending::AccountPubkeys {
            reserve: money_market.reserve_pubkey,
            reserve_liquidity_supply: reserve.liquidity.supply_pubkey,
            reserve_liquidity_oracle: reserve.liquidity.oracle_pubkey,
            lending_market: money_market.market_pubkey,
        });

    context.warp_to_slot(5).unwrap();
    pyth_oracle.update(&mut context, 5).await;

    let money_market_program_id = &spl_token_lending::id();

    let collateral_mint = mm_pool.token_mint_pubkey;
    let liquidity_mint = get_liquidity_mint().1;
    let mm_pool_collateral_mint = mm_pool.pool_mint.pubkey();

    let withdraw_accounts =
        integrations::withdraw_accounts(money_market_program_id, &money_market_pubkeys);

    let tx = Transaction::new_signed_with_payer(
        &[everlend_depositor::instruction::withdraw(
            &everlend_depositor::id(),
            &registry.keypair.pubkey(),
            &Pubkey::new_unique(),
            &income_pool_market.keypair.pubkey(),
            &income_pool.token_account.pubkey(),
            &mm_pool_market.keypair.pubkey(),
            &mm_pool.token_account.pubkey(),
            &mm_pool_collateral_mint,
            &collateral_mint,
            &liquidity_mint,
            money_market_program_id,
            withdraw_accounts,
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
async fn fail_with_invalid_income_pool_market() {
    let (
        mut context,
        money_market,
        pyth_oracle,
        registry,
        _general_pool_market,
        _,
        _general_pool_borrow_authority,
        _,
        income_pool,
        mm_pool_market,
        mm_pool,
        liquidity_provider,
        test_depositor,
    ) = setup().await;

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

    context.warp_to_slot(5).unwrap();
    pyth_oracle.update(&mut context, 5).await;

    let money_market_program_id = &spl_token_lending::id();

    let money_market_pubkeys =
        MoneyMarketPubkeys::SPL(integrations::spl_token_lending::AccountPubkeys {
            reserve: money_market.reserve_pubkey,
            reserve_liquidity_supply: reserve.liquidity.supply_pubkey,
            reserve_liquidity_oracle: reserve.liquidity.oracle_pubkey,
            lending_market: money_market.market_pubkey,
        });

    let collateral_mint = mm_pool.token_mint_pubkey;
    let liquidity_mint = get_liquidity_mint().1;
    let mm_pool_collateral_mint = mm_pool.pool_mint.pubkey();

    let withdraw_accounts =
        integrations::withdraw_accounts(money_market_program_id, &money_market_pubkeys);

    let tx = Transaction::new_signed_with_payer(
        &[everlend_depositor::instruction::withdraw(
            &everlend_depositor::id(),
            &registry.keypair.pubkey(),
            &test_depositor.depositor.pubkey(),
            &Pubkey::new_unique(),
            &income_pool.token_account.pubkey(),
            &mm_pool_market.keypair.pubkey(),
            &mm_pool.token_account.pubkey(),
            &mm_pool_collateral_mint,
            &collateral_mint,
            &liquidity_mint,
            money_market_program_id,
            withdraw_accounts,
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
async fn fail_with_invalid_income_pool_token_account() {
    let (
        mut context,
        money_market,
        pyth_oracle,
        registry,
        _general_pool_market,
        _,
        _general_pool_borrow_authority,
        income_pool_market,
        _,
        mm_pool_market,
        mm_pool,
        liquidity_provider,
        test_depositor,
    ) = setup().await;

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

    context.warp_to_slot(5).unwrap();
    pyth_oracle.update(&mut context, 5).await;

    let money_market_program_id = &spl_token_lending::id();

    let money_market_pubkeys =
        MoneyMarketPubkeys::SPL(integrations::spl_token_lending::AccountPubkeys {
            reserve: money_market.reserve_pubkey,
            reserve_liquidity_supply: reserve.liquidity.supply_pubkey,
            reserve_liquidity_oracle: reserve.liquidity.oracle_pubkey,
            lending_market: money_market.market_pubkey,
        });

    let collateral_mint = mm_pool.token_mint_pubkey;
    let liquidity_mint = get_liquidity_mint().1;
    let mm_pool_collateral_mint = mm_pool.pool_mint.pubkey();

    let withdraw_accounts =
        integrations::withdraw_accounts(money_market_program_id, &money_market_pubkeys);

    let tx = Transaction::new_signed_with_payer(
        &[everlend_depositor::instruction::withdraw(
            &everlend_depositor::id(),
            &registry.keypair.pubkey(),
            &test_depositor.depositor.pubkey(),
            &income_pool_market.keypair.pubkey(),
            &Pubkey::new_unique(),
            &mm_pool_market.keypair.pubkey(),
            &mm_pool.token_account.pubkey(),
            &mm_pool_collateral_mint,
            &collateral_mint,
            &liquidity_mint,
            money_market_program_id,
            withdraw_accounts,
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
async fn fail_with_invalid_mm_pool_market() {
    let (
        mut context,
        money_market,
        pyth_oracle,
        registry,
        _general_pool_market,
        _,
        _general_pool_borrow_authority,
        income_pool_market,
        income_pool,
        _,
        mm_pool,
        _,
        test_depositor,
    ) = setup().await;

    let reserve = money_market.get_reserve_data(&mut context).await;

    let money_market_pubkeys =
        MoneyMarketPubkeys::SPL(integrations::spl_token_lending::AccountPubkeys {
            reserve: money_market.reserve_pubkey,
            reserve_liquidity_supply: reserve.liquidity.supply_pubkey,
            reserve_liquidity_oracle: reserve.liquidity.oracle_pubkey,
            lending_market: money_market.market_pubkey,
        });

    context.warp_to_slot(5).unwrap();
    pyth_oracle.update(&mut context, 5).await;

    let money_market_program_id = &spl_token_lending::id();

    let collateral_mint = mm_pool.token_mint_pubkey;
    let liquidity_mint = get_liquidity_mint().1;
    let mm_pool_collateral_mint = mm_pool.pool_mint.pubkey();

    let withdraw_accounts =
        integrations::withdraw_accounts(money_market_program_id, &money_market_pubkeys);

    let tx = Transaction::new_signed_with_payer(
        &[everlend_depositor::instruction::withdraw(
            &everlend_depositor::id(),
            &registry.keypair.pubkey(),
            &test_depositor.depositor.pubkey(),
            &income_pool_market.keypair.pubkey(),
            &income_pool.token_account.pubkey(),
            &Pubkey::new_unique(),
            &mm_pool.token_account.pubkey(),
            &mm_pool_collateral_mint,
            &collateral_mint,
            &liquidity_mint,
            money_market_program_id,
            withdraw_accounts,
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
        _general_pool_market,
        _,
        _general_pool_borrow_authority,
        income_pool_market,
        income_pool,
        mm_pool_market,
        mm_pool,
        _,
        test_depositor,
    ) = setup().await;

    let reserve = money_market.get_reserve_data(&mut context).await;

    let money_market_pubkeys =
        MoneyMarketPubkeys::SPL(integrations::spl_token_lending::AccountPubkeys {
            reserve: money_market.reserve_pubkey,
            reserve_liquidity_supply: reserve.liquidity.supply_pubkey,
            reserve_liquidity_oracle: reserve.liquidity.oracle_pubkey,
            lending_market: money_market.market_pubkey,
        });

    context.warp_to_slot(5).unwrap();
    pyth_oracle.update(&mut context, 5).await;

    let money_market_program_id = &spl_token_lending::id();

    let collateral_mint = mm_pool.token_mint_pubkey;
    let liquidity_mint = get_liquidity_mint().1;
    let mm_pool_collateral_mint = mm_pool.pool_mint.pubkey();

    let withdraw_accounts =
        integrations::withdraw_accounts(money_market_program_id, &money_market_pubkeys);

    let tx = Transaction::new_signed_with_payer(
        &[everlend_depositor::instruction::withdraw(
            &everlend_depositor::id(),
            &registry.keypair.pubkey(),
            &test_depositor.depositor.pubkey(),
            &income_pool_market.keypair.pubkey(),
            &income_pool.token_account.pubkey(),
            &mm_pool_market.keypair.pubkey(),
            &Pubkey::new_unique(),
            &mm_pool_collateral_mint,
            &collateral_mint,
            &liquidity_mint,
            money_market_program_id,
            withdraw_accounts,
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
        _general_pool_market,
        _,
        _general_pool_borrow_authority,
        income_pool_market,
        income_pool,
        mm_pool_market,
        mm_pool,
        _,
        test_depositor,
    ) = setup().await;

    let reserve = money_market.get_reserve_data(&mut context).await;

    let money_market_pubkeys =
        MoneyMarketPubkeys::SPL(integrations::spl_token_lending::AccountPubkeys {
            reserve: money_market.reserve_pubkey,
            reserve_liquidity_supply: reserve.liquidity.supply_pubkey,
            reserve_liquidity_oracle: reserve.liquidity.oracle_pubkey,
            lending_market: money_market.market_pubkey,
        });

    context.warp_to_slot(5).unwrap();
    pyth_oracle.update(&mut context, 5).await;

    let money_market_program_id = &spl_token_lending::id();

    let collateral_mint = mm_pool.token_mint_pubkey;
    let liquidity_mint = get_liquidity_mint().1;
    let mm_pool_collateral_mint = Pubkey::new_unique();

    let withdraw_accounts =
        integrations::withdraw_accounts(money_market_program_id, &money_market_pubkeys);

    let tx = Transaction::new_signed_with_payer(
        &[everlend_depositor::instruction::withdraw(
            &everlend_depositor::id(),
            &registry.keypair.pubkey(),
            &test_depositor.depositor.pubkey(),
            &income_pool_market.keypair.pubkey(),
            &income_pool.token_account.pubkey(),
            &mm_pool_market.keypair.pubkey(),
            &mm_pool.token_account.pubkey(),
            &mm_pool_collateral_mint,
            &collateral_mint,
            &liquidity_mint,
            money_market_program_id,
            withdraw_accounts,
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
async fn fail_with_invalid_collateral_mint() {
    let (
        mut context,
        money_market,
        pyth_oracle,
        registry,
        _general_pool_market,
        _,
        _general_pool_borrow_authority,
        income_pool_market,
        income_pool,
        mm_pool_market,
        mm_pool,
        _,
        test_depositor,
    ) = setup().await;

    let reserve = money_market.get_reserve_data(&mut context).await;

    let money_market_pubkeys =
        MoneyMarketPubkeys::SPL(integrations::spl_token_lending::AccountPubkeys {
            reserve: money_market.reserve_pubkey,
            reserve_liquidity_supply: reserve.liquidity.supply_pubkey,
            reserve_liquidity_oracle: reserve.liquidity.oracle_pubkey,
            lending_market: money_market.market_pubkey,
        });

    context.warp_to_slot(5).unwrap();
    pyth_oracle.update(&mut context, 5).await;

    let money_market_program_id = &spl_token_lending::id();

    let collateral_mint = Pubkey::new_unique();
    let liquidity_mint = get_liquidity_mint().1;
    let mm_pool_collateral_mint = mm_pool.pool_mint.pubkey();

    let withdraw_accounts =
        integrations::withdraw_accounts(money_market_program_id, &money_market_pubkeys);

    let tx = Transaction::new_signed_with_payer(
        &[everlend_depositor::instruction::withdraw(
            &everlend_depositor::id(),
            &registry.keypair.pubkey(),
            &test_depositor.depositor.pubkey(),
            &income_pool_market.keypair.pubkey(),
            &income_pool.token_account.pubkey(),
            &mm_pool_market.keypair.pubkey(),
            &mm_pool.token_account.pubkey(),
            &mm_pool_collateral_mint,
            &collateral_mint,
            &liquidity_mint,
            money_market_program_id,
            withdraw_accounts,
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
        _general_pool_market,
        _,
        _general_pool_borrow_authority,
        income_pool_market,
        income_pool,
        mm_pool_market,
        mm_pool,
        _,
        test_depositor,
    ) = setup().await;

    let reserve = money_market.get_reserve_data(&mut context).await;

    let money_market_pubkeys =
        MoneyMarketPubkeys::SPL(integrations::spl_token_lending::AccountPubkeys {
            reserve: money_market.reserve_pubkey,
            reserve_liquidity_supply: reserve.liquidity.supply_pubkey,
            reserve_liquidity_oracle: reserve.liquidity.oracle_pubkey,
            lending_market: money_market.market_pubkey,
        });

    context.warp_to_slot(5).unwrap();
    pyth_oracle.update(&mut context, 5).await;

    let money_market_program_id = &spl_token_lending::id();

    let collateral_mint = mm_pool.token_mint_pubkey;
    let liquidity_mint = Pubkey::new_unique();
    let mm_pool_collateral_mint = mm_pool.pool_mint.pubkey();

    let withdraw_accounts =
        integrations::withdraw_accounts(money_market_program_id, &money_market_pubkeys);

    let tx = Transaction::new_signed_with_payer(
        &[everlend_depositor::instruction::withdraw(
            &everlend_depositor::id(),
            &registry.keypair.pubkey(),
            &test_depositor.depositor.pubkey(),
            &income_pool_market.keypair.pubkey(),
            &income_pool.token_account.pubkey(),
            &mm_pool_market.keypair.pubkey(),
            &mm_pool.token_account.pubkey(),
            &mm_pool_collateral_mint,
            &collateral_mint,
            &liquidity_mint,
            money_market_program_id,
            withdraw_accounts,
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
async fn fail_with_invalid_mm_program_id() {
    let (
        mut context,
        money_market,
        pyth_oracle,
        registry,
        _general_pool_market,
        _,
        _general_pool_borrow_authority,
        income_pool_market,
        income_pool,
        mm_pool_market,
        mm_pool,
        _,
        test_depositor,
    ) = setup().await;

    let reserve = money_market.get_reserve_data(&mut context).await;

    let money_market_pubkeys =
        MoneyMarketPubkeys::SPL(integrations::spl_token_lending::AccountPubkeys {
            reserve: money_market.reserve_pubkey,
            reserve_liquidity_supply: reserve.liquidity.supply_pubkey,
            reserve_liquidity_oracle: reserve.liquidity.oracle_pubkey,
            lending_market: money_market.market_pubkey,
        });

    context.warp_to_slot(5).unwrap();
    pyth_oracle.update(&mut context, 5).await;

    let money_market_program_id = &Pubkey::new_unique();

    let collateral_mint = mm_pool.token_mint_pubkey;
    let liquidity_mint = get_liquidity_mint().1;
    let mm_pool_collateral_mint = mm_pool.pool_mint.pubkey();

    let withdraw_accounts =
        integrations::withdraw_accounts(money_market_program_id, &money_market_pubkeys);

    let tx = Transaction::new_signed_with_payer(
        &[everlend_depositor::instruction::withdraw(
            &everlend_depositor::id(),
            &registry.keypair.pubkey(),
            &test_depositor.depositor.pubkey(),
            &income_pool_market.keypair.pubkey(),
            &income_pool.token_account.pubkey(),
            &mm_pool_market.keypair.pubkey(),
            &mm_pool.token_account.pubkey(),
            &mm_pool_collateral_mint,
            &collateral_mint,
            &liquidity_mint,
            money_market_program_id,
            withdraw_accounts,
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
async fn fail_with_invalid_withdraw_accounts() {
    let (
        mut context,
        money_market,
        pyth_oracle,
        registry,
        _general_pool_market,
        _,
        _general_pool_borrow_authority,
        income_pool_market,
        income_pool,
        mm_pool_market,
        mm_pool,
        _,
        test_depositor,
    ) = setup().await;

    let reserve = money_market.get_reserve_data(&mut context).await;

    let money_market_pubkeys =
        MoneyMarketPubkeys::SPL(integrations::spl_token_lending::AccountPubkeys {
            reserve: money_market.reserve_pubkey,
            reserve_liquidity_supply: reserve.liquidity.supply_pubkey,
            reserve_liquidity_oracle: reserve.liquidity.oracle_pubkey,
            lending_market: money_market.market_pubkey,
        });

    context.warp_to_slot(5).unwrap();
    pyth_oracle.update(&mut context, 5).await;

    let money_market_program_id = &spl_token_lending::id();

    let collateral_mint = mm_pool.token_mint_pubkey;
    let liquidity_mint = get_liquidity_mint().1;
    let mm_pool_collateral_mint = mm_pool.pool_mint.pubkey();

    let withdraw_accounts =
        integrations::withdraw_accounts(&Pubkey::new_unique(), &money_market_pubkeys);

    let tx = Transaction::new_signed_with_payer(
        &[everlend_depositor::instruction::withdraw(
            &everlend_depositor::id(),
            &registry.keypair.pubkey(),
            &test_depositor.depositor.pubkey(),
            &income_pool_market.keypair.pubkey(),
            &income_pool.token_account.pubkey(),
            &mm_pool_market.keypair.pubkey(),
            &mm_pool.token_account.pubkey(),
            &mm_pool_collateral_mint,
            &collateral_mint,
            &liquidity_mint,
            money_market_program_id,
            withdraw_accounts,
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
        TransactionError::InstructionError(0, InstructionError::MissingAccount)
    );
}
