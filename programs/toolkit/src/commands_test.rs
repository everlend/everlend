use anyhow::Context;
use core::time;
use everlend_depositor::{
    find_rebalancing_program_address,
    state::{Rebalancing, RebalancingOperation},
};
use everlend_general_pool::state::WITHDRAW_DELAY;
use everlend_liquidity_oracle::state::DistributionArray;
use everlend_registry::{
    find_config_program_address,
    state::{RegistryConfig, RegistryPrograms},
};
use everlend_utils::integrations::{self, MoneyMarketPubkeys};
use solana_account_decoder::parse_token::UiTokenAmount;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;
use solana_sdk::{signature::Keypair, signer::Signer};
use std::{str::FromStr, thread};

use crate::{
    accounts_config::InitializedAccounts,
    depositor, distribution,
    general_pool::{self, get_withdrawal_request_accounts},
    larix_liquidity_mining, liquidity_oracle,
    utils::Config,
};

pub async fn command_run_test(
    config: &Config,
    accounts_path: &str,
    case: Option<String>,
) -> anyhow::Result<()> {
    println!("Run {:?}", case);

    let default_accounts = config.get_default_accounts();
    let initialized_accounts = InitializedAccounts::load(accounts_path).unwrap_or_default();
    println!("default_accounts = {:#?}", default_accounts);

    let InitializedAccounts {
        payer: _,
        registry,
        general_pool_market,
        income_pool_market,
        mm_pool_markets: _,
        collateral_pool_markets,
        token_accounts,
        liquidity_oracle,
        depositor,
        larix_mining: _,
    } = initialized_accounts;

    let (registry_config_pubkey, _) =
        find_config_program_address(&everlend_registry::id(), &registry);
    let registry_config_account = config.rpc_client.get_account(&registry_config_pubkey)?;
    let registry_config = RegistryConfig::unpack(&registry_config_account.data).unwrap();
    let programs = RegistryPrograms::unpack_unchecked(&registry_config_account.data).unwrap();

    println!("registry_config = {:#?}", registry_config);
    println!("programs = {:#?}", programs);

    let sol = token_accounts.get("SOL").unwrap();

    let sol_oracle = default_accounts.sol_oracle;
    let port_finance_pubkeys = integrations::spl_token_lending::AccountPubkeys {
        reserve: default_accounts.port_finance_reserve_sol,
        reserve_liquidity_supply: default_accounts.port_finance_reserve_sol_supply,
        reserve_liquidity_oracle: sol_oracle,
        lending_market: default_accounts.port_finance_lending_market,
    };
    let larix_pubkeys = integrations::larix::AccountPubkeys {
        reserve: default_accounts.larix_reserve_sol,
        reserve_liquidity_supply: default_accounts.larix_reserve_sol_supply,
        reserve_liquidity_oracle: sol_oracle,
        lending_market: default_accounts.larix_lending_market,
    };

    let solend_pubkeys = integrations::solend::AccountPubkeys {
        reserve: default_accounts.solend_reserve_sol,
        reserve_liquidity_supply: default_accounts
            .solend_reserve_sol_supply
            .context("`solend_reserve_sol_supply` invalid value")
            .unwrap(),
        reserve_liquidity_pyth_oracle: default_accounts
            .solend_reserve_pyth_oracle
            .context("`solend_reserve_pyth_oracle` invalid value")
            .unwrap(),
        reserve_liquidity_switchboard_oracle: default_accounts
            .solend_reserve_switchboard_oracle
            .context("`solend_reserve_switchboard_oracle` invalid value")
            .unwrap(),
        lending_market: default_accounts.solend_lending_market,
    };

    let get_balance = |pk: &Pubkey| config.rpc_client.get_token_account_balance(pk);

    let print_balance = |v: (UiTokenAmount, UiTokenAmount)| {
        println!(
            "Balance:\n\
             - liquidity_transit: {}\n\
             - general_pool_token_account: {}",
            v.0.amount, v.1.amount
        );
    };

    let update_token_distribution = |d: DistributionArray| {
        liquidity_oracle::update_token_distribution(config, &liquidity_oracle, &sol.mint, &d)
    };

    let withdraw_requests =
        || get_withdrawal_request_accounts(config, &general_pool_market, &sol.mint);

    let start_rebalancing = || {
        println!("Rebalancing");
        depositor::start_rebalancing(
            config,
            &registry,
            &depositor,
            &sol.mint,
            &general_pool_market,
            &sol.general_pool_token_account,
            &liquidity_oracle,
            false,
        )
    };

    let refresh_income = || {
        println!("Rebalancing (Refresh income)");
        depositor::start_rebalancing(
            config,
            &registry,
            &depositor,
            &sol.mint,
            &general_pool_market,
            &sol.general_pool_token_account,
            &liquidity_oracle,
            true,
        )
    };

    let deposit = |i: usize| {
        println!("Rebalancing: Deposit: {}", i);
        let pubkeys = match i {
            0 => MoneyMarketPubkeys::SPL(port_finance_pubkeys.clone()),
            1 => MoneyMarketPubkeys::Larix(larix_pubkeys.clone()),
            2 => MoneyMarketPubkeys::Solend(solend_pubkeys.clone()),
            _ => panic!("wrong pubkey idx"),
        };

        depositor::deposit(
            config,
            &registry,
            &depositor,
            &collateral_pool_markets[i],
            &sol.mm_pools[i].pool_token_account,
            &sol.mint,
            &sol.mm_pools[i].token_mint,
            &programs.money_market_program_ids[i],
            integrations::deposit_accounts(&programs.money_market_program_ids[i], &pubkeys),
        )
    };

    let withdraw = |i| {
        println!("Rebalancing: Withdraw: {}", i);
        let pubkeys = match i {
            0 => MoneyMarketPubkeys::SPL(port_finance_pubkeys.clone()),
            1 => MoneyMarketPubkeys::Larix(larix_pubkeys.clone()),
            2 => MoneyMarketPubkeys::Solend(solend_pubkeys.clone()),
            _ => panic!("wrong pubkey idx"),
        };

        depositor::withdraw(
            config,
            &registry,
            &depositor,
            &income_pool_market,
            &sol.income_pool_token_account,
            &collateral_pool_markets[i],
            &sol.mm_pools[i].pool_token_account,
            &sol.mm_pools[i].token_mint,
            &sol.mint,
            &programs.money_market_program_ids[i],
            integrations::withdraw_accounts(&programs.money_market_program_ids[i], &pubkeys),
        )
    };

    let complete_rebalancing = |rebalancing: Option<Rebalancing>| -> anyhow::Result<()> {
        let rebalancing = rebalancing.or_else(|| {
            let (rebalancing_pubkey, _) =
                find_rebalancing_program_address(&everlend_depositor::id(), &depositor, &sol.mint);
            config
                .rpc_client
                .get_account(&rebalancing_pubkey)
                .ok()
                .and_then(|a| Rebalancing::unpack(&a.data).ok())
        });

        if rebalancing.is_none() {
            return Ok(());
        }

        let rebalancing = rebalancing.unwrap();
        println!("{:#?}", rebalancing);
        print_balance((
            get_balance(&sol.liquidity_transit)?,
            get_balance(&sol.general_pool_token_account)?,
        ));

        for step in rebalancing
            .steps
            .iter()
            .filter(|&step| step.executed_at.is_none())
        {
            match step.operation {
                RebalancingOperation::Deposit => deposit(step.money_market_index.into())?,
                RebalancingOperation::Withdraw => withdraw(step.money_market_index.into())?,
            }
        }

        print_balance((
            get_balance(&sol.liquidity_transit)?,
            get_balance(&sol.general_pool_token_account)?,
        ));

        Ok(())
    };

    let general_pool_deposit = |a: u64| {
        println!("Deposit liquidity");
        general_pool::deposit(
            config,
            &registry,
            &general_pool_market,
            &sol.general_pool,
            &sol.liquidity_token_account,
            &sol.collateral_token_account,
            &sol.general_pool_token_account,
            &sol.general_pool_mint,
            a,
        )
    };

    let general_pool_withdraw_request = |a: u64| {
        println!("Withdraw request");
        general_pool::withdraw_request(
            config,
            &registry,
            &general_pool_market,
            &sol.general_pool,
            &sol.collateral_token_account,
            &sol.liquidity_token_account,
            &sol.general_pool_token_account,
            &sol.mint,
            &sol.general_pool_mint,
            a,
        )
    };

    let delay = |secs| {
        println!("Waiting {} secs for ticket...", secs);
        thread::sleep(time::Duration::from_secs(secs))
    };

    let general_pool_withdraw = || {
        println!("Withdraw");
        general_pool::withdraw(
            config,
            &general_pool_market,
            &sol.general_pool,
            &sol.liquidity_token_account,
            &sol.general_pool_token_account,
            &sol.mint,
            &sol.general_pool_mint,
        )
    };

    complete_rebalancing(None)?;

    match case.as_deref() {
        Some("first") => {
            general_pool_deposit(1000)?;

            update_token_distribution(distribution!([1000000000, 0]))?;
            let (_, rebalancing) = start_rebalancing()?;
            complete_rebalancing(Some(rebalancing))?;

            update_token_distribution(distribution!([959876767, 0]))?;
            let (_, rebalancing) = start_rebalancing()?;
            complete_rebalancing(Some(rebalancing))?;

            update_token_distribution(distribution!([959876767, 0]))?;
            let (_, rebalancing) = start_rebalancing()?;
            complete_rebalancing(Some(rebalancing))?;

            update_token_distribution(distribution!([959876767, 0]))?;
            let (_, rebalancing) = start_rebalancing()?;
            println!("{:#?}", rebalancing);
        }
        Some("second") => {
            general_pool_deposit(1000)?;

            update_token_distribution(distribution!([500000000, 500000000]))?;
            let (_, rebalancing) = start_rebalancing()?;
            complete_rebalancing(Some(rebalancing))?;

            general_pool_deposit(10)?;

            update_token_distribution(distribution!([900000000, 100000000]))?;
            let (_, rebalancing) = start_rebalancing()?;
            complete_rebalancing(Some(rebalancing))?;
        }
        Some("third") => {
            general_pool_deposit(1000)?;

            update_token_distribution(distribution!([100000000, 100000000, 800000000]))?;
            let (_, rebalancing) = start_rebalancing()?;
            complete_rebalancing(Some(rebalancing))?;

            general_pool_deposit(10)?;

            update_token_distribution(distribution!([0, 300000000, 700000000]))?;
            let (_, rebalancing) = start_rebalancing()?;
            complete_rebalancing(Some(rebalancing))?;
        }
        Some("larix") => {
            general_pool_deposit(1000)?;

            update_token_distribution(distribution!([0, 0, 1000000000]))?;
            let (_, rebalancing) = start_rebalancing()?;
            complete_rebalancing(Some(rebalancing))?;
        }
        Some("solend") => {
            general_pool_deposit(1000)?;

            update_token_distribution(distribution!([0, 0, 1000000000]))?;
            let (_, rebalancing) = start_rebalancing()?;
            complete_rebalancing(Some(rebalancing))?;
        }
        Some("zero-distribution") => {
            general_pool_deposit(1000)?;

            update_token_distribution(distribution!([0, 0]))?;
            let (_, rebalancing) = start_rebalancing()?;
            complete_rebalancing(Some(rebalancing))?;
        }
        Some("deposit") => {
            general_pool_deposit(1000)?;

            update_token_distribution(distribution!([1000000000, 0]))?;
            let (_, rebalancing) = start_rebalancing()?;
            complete_rebalancing(Some(rebalancing))?;

            general_pool_deposit(1000)?;

            update_token_distribution(distribution!([1000000000, 0]))?;
            let (_, rebalancing) = start_rebalancing()?;
            println!("{:#?}", rebalancing);
        }
        Some("full") => {
            general_pool_deposit(1000)?;

            update_token_distribution(distribution!([1000000000, 0]))?;
            let (_, rebalancing) = start_rebalancing()?;
            complete_rebalancing(Some(rebalancing))?;

            general_pool_withdraw_request(100)?;
            let withdraw_requests = withdraw_requests()?;
            println!("{:#?}", withdraw_requests);

            update_token_distribution(distribution!([1000000000, 0]))?;
            let (_, rebalancing) = start_rebalancing()?;
            complete_rebalancing(Some(rebalancing))?;

            update_token_distribution(distribution!([1000000000, 0]))?;
            let (_, rebalancing) = start_rebalancing()?;
            println!("{:#?}", rebalancing);

            delay(WITHDRAW_DELAY / 2);
            general_pool_withdraw()?;

            update_token_distribution(distribution!([1000000000, 0]))?;
            let (_, rebalancing) = start_rebalancing()?;
            println!("{:#?}", rebalancing);
        }
        Some("withdraw") => {
            general_pool_withdraw()?;
        }
        Some("11") => {
            general_pool_deposit(4321)?;

            update_token_distribution(distribution!([10, 10]))?;
            let (_, rebalancing) = start_rebalancing()?;
            complete_rebalancing(Some(rebalancing))?;

            update_token_distribution(distribution!([10, 20]))?;
            let (_, rebalancing) = start_rebalancing()?;
            complete_rebalancing(Some(rebalancing))?;
        }
        Some("empty") => {
            update_token_distribution(distribution!([1000000000, 0]))?;
            start_rebalancing()?;
        }
        Some("refresh-income") => {
            general_pool_deposit(1000)?;

            update_token_distribution(distribution!([1000000000, 0]))?;
            let (_, rebalancing) = start_rebalancing()?;
            complete_rebalancing(Some(rebalancing))?;

            let (_, rebalancing) = refresh_income()?;
            println!("{:#?}", rebalancing);
        }
        None => {
            general_pool_deposit(1000)?;

            update_token_distribution(distribution!([500000000, 500000000]))?;
            let (_, rebalancing) = start_rebalancing()?;
            complete_rebalancing(Some(rebalancing))?;

            general_pool_withdraw_request(100)?;

            update_token_distribution(distribution!([300000000, 600000000]))?;
            let (_, rebalancing) = start_rebalancing()?;
            complete_rebalancing(Some(rebalancing))?;

            update_token_distribution(distribution!([0, 1000000000]))?;
            let (_, rebalancing) = start_rebalancing()?;
            delay(WITHDRAW_DELAY / 2);
            general_pool_withdraw()?;
            complete_rebalancing(Some(rebalancing))?;

            update_token_distribution(distribution!([100000000, 0]))?;
            let (_, rebalancing) = start_rebalancing()?;
            complete_rebalancing(Some(rebalancing))?;
        }
        _ => {}
    }

    println!("Finished!");

    Ok(())
}

pub async fn command_test_larix_mining_raw(config: &Config) -> anyhow::Result<()> {
    // to get this id do "spl-token wrap 1" in your terminal
    let source_sol = Pubkey::from_str("44mZcJKT4HaaP2jWzdW1DHgu182Tk21ep6qVUJYYXh6q").unwrap();
    let amount = 20_000_000;
    let mining_account = Keypair::new();
    let collateral_transit = Keypair::new();
    let devidends_account = Keypair::new();
    let withdraw_account = Keypair::new();
    larix_liquidity_mining::init_mining_accounts(&config, &mining_account)?;
    println!("init mining accounts finished");
    larix_liquidity_mining::deposit_liquidity(&config, amount, &source_sol, &collateral_transit)?;
    println!("deposit liquidity finished");
    larix_liquidity_mining::deposit_collateral(
        &config,
        amount,
        &mining_account.pubkey(),
        &collateral_transit.pubkey(),
    )?;
    println!("deposit collateral finished");
    thread::sleep(time::Duration::from_secs(2));
    larix_liquidity_mining::claim_mining(&config, &devidends_account, &mining_account.pubkey())?;
    println!("claim dividends finished");
    larix_liquidity_mining::withdraw_collateral(
        &config,
        10_000_000,
        &withdraw_account,
        &mining_account.pubkey(),
    )?;
    println!("withdraw collateral finished");
    Ok(())
}
