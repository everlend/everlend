mod accounts_config;
mod depositor;
mod general_pool;
mod income_pools;
mod liquidity_oracle;
mod registry;
mod ulp;
mod utils;

use accounts_config::*;
use clap::{
    crate_description, crate_name, crate_version, value_t, App, AppSettings, Arg, SubCommand,
};
use core::time;
use everlend_depositor::{
    find_rebalancing_program_address,
    state::{Rebalancing, RebalancingOperation},
};
use everlend_general_pool::state::WITHDRAW_DELAY;
use everlend_liquidity_oracle::state::DistributionArray;
use everlend_registry::{
    find_config_program_address,
    state::{RegistryConfig, SetRegistryConfigParams, TOTAL_DISTRIBUTIONS},
};
use everlend_utils::integrations::{self, MoneyMarket, MoneyMarketPubkeys};
use general_pool::get_withdrawal_requests;
use regex::Regex;
use solana_account_decoder::parse_token::UiTokenAmount;
use solana_clap_utils::{
    fee_payer::fee_payer_arg,
    input_parsers::{keypair_of, value_of},
    input_validators::{is_amount, is_keypair},
    keypair::signer_from_path,
};
use solana_client::rpc_client::RpcClient;
use solana_program::{program_pack::Pack, pubkey::Pubkey};
use solana_sdk::{commitment_config::CommitmentConfig, signature::Keypair};
use spl_associated_token_account::get_associated_token_address;
use std::{collections::HashMap, process::exit, thread};
use utils::*;

use crate::general_pool::{get_general_pool_market, get_withdrawal_request_accounts};

/// Generates fixed distribution from slice
#[macro_export]
macro_rules! distribution {
    ($distribuition:expr) => {{
        let mut new_distribuition = DistributionArray::default();
        new_distribuition[..$distribuition.len()].copy_from_slice(&$distribuition);
        new_distribuition
    }};
}

pub fn url_to_moniker(url: &str) -> String {
    let re = Regex::new(r"devnet|mainnet|localhost|testnet").unwrap();
    let cap = &re.captures(url).unwrap()[0];

    match cap {
        "mainnet" => "mainnet-beta",
        _ => cap,
    }
    .to_string()
}

async fn command_create_registry(config: &Config, keypair: Option<Keypair>) -> anyhow::Result<()> {
    let payer_pubkey = config.fee_payer.pubkey();
    println!("Fee payer: {}", payer_pubkey);

    let default_accounts = get_default_accounts(config);
    let mut initialiazed_accounts = get_initialized_accounts(config);

    let registry_pubkey = registry::init(config, keypair)?;
    let mut registry_config = SetRegistryConfigParams {
        general_pool_program_id: everlend_general_pool::id(),
        ulp_program_id: everlend_ulp::id(),
        liquidity_oracle_program_id: everlend_liquidity_oracle::id(),
        depositor_program_id: everlend_depositor::id(),
        income_pools_program_id: everlend_income_pools::id(),
        money_market_program_ids: [Pubkey::default(); TOTAL_DISTRIBUTIONS],
        refresh_income_interval: REFRESH_INCOME_INTERVAL,
    };

    registry_config.money_market_program_ids[0] = default_accounts.port_finance_program_id;
    registry_config.money_market_program_ids[1] = default_accounts.larix_program_id;
    registry_config.money_market_program_ids[2] = default_accounts.solend_program_id;

    println!("registry_config = {:#?}", registry_config);

    registry::set_registry_config(config, &registry_pubkey, registry_config)?;

    initialiazed_accounts.payer = payer_pubkey;
    initialiazed_accounts.registry = registry_pubkey;

    initialiazed_accounts
        .save(&format!("accounts.{}.yaml", config.network))
        .unwrap();

    Ok(())
}

async fn command_create_general_pool_market(
    config: &Config,
    keypair: Option<Keypair>,
) -> anyhow::Result<()> {
    let mut initialiazed_accounts = get_initialized_accounts(config);

    let general_pool_market_pubkey = general_pool::create_market(config, keypair)?;

    initialiazed_accounts.general_pool_market = general_pool_market_pubkey;

    initialiazed_accounts
        .save(&format!("accounts.{}.yaml", config.network))
        .unwrap();

    Ok(())
}

async fn command_create_income_pool_market(
    config: &Config,
    keypair: Option<Keypair>,
) -> anyhow::Result<()> {
    let mut initialiazed_accounts = get_initialized_accounts(config);

    let income_pool_market_pubkey =
        income_pools::create_market(config, keypair, &initialiazed_accounts.general_pool_market)?;

    initialiazed_accounts.income_pool_market = income_pool_market_pubkey;

    initialiazed_accounts
        .save(&format!("accounts.{}.yaml", config.network))
        .unwrap();

    Ok(())
}

async fn command_create_mm_pool_market(
    config: &Config,
    keypair: Option<Keypair>,
    money_market: MoneyMarket,
) -> anyhow::Result<()> {
    let mut initialiazed_accounts = get_initialized_accounts(config);

    let mm_pool_market_pubkey = ulp::create_market(config, keypair)?;

    initialiazed_accounts.mm_pool_markets[money_market as usize] = mm_pool_market_pubkey;

    initialiazed_accounts
        .save(&format!("accounts.{}.yaml", config.network))
        .unwrap();

    Ok(())
}

async fn command_create_liquidity_oracle(
    config: &Config,
    keypair: Option<Keypair>,
) -> anyhow::Result<()> {
    let mut initialiazed_accounts = get_initialized_accounts(config);

    let liquidity_oracle_pubkey = liquidity_oracle::init(config, keypair)?;

    initialiazed_accounts.liquidity_oracle = liquidity_oracle_pubkey;

    initialiazed_accounts
        .save(&format!("accounts.{}.yaml", config.network))
        .unwrap();

    Ok(())
}

async fn command_create_depositor(config: &Config, keypair: Option<Keypair>) -> anyhow::Result<()> {
    let mut initialiazed_accounts = get_initialized_accounts(config);

    let depositor_pubkey = depositor::init(
        config,
        &initialiazed_accounts.registry,
        keypair,
        &initialiazed_accounts.general_pool_market,
        &initialiazed_accounts.income_pool_market,
        &initialiazed_accounts.liquidity_oracle,
    )?;

    initialiazed_accounts.depositor = depositor_pubkey;

    initialiazed_accounts
        .save(&format!("accounts.{}.yaml", config.network))
        .unwrap();

    Ok(())
}

async fn command_create_token_accounts(
    config: &Config,
    required_mints: Vec<&str>,
) -> anyhow::Result<()> {
    let payer_pubkey = config.fee_payer.pubkey();
    let default_accounts = get_default_accounts(config);
    let mut initialiazed_accounts = get_initialized_accounts(config);

    let mint_map = HashMap::from([
        ("SOL".to_string(), default_accounts.sol_mint),
        ("USDC".to_string(), default_accounts.usdc_mint),
        ("USDT".to_string(), default_accounts.usdt_mint),
    ]);

    let collateral_mint_map = HashMap::from([
        ("SOL".to_string(), default_accounts.sol_collateral),
        ("USDC".to_string(), default_accounts.usdc_collateral),
        ("USDT".to_string(), default_accounts.usdt_collateral),
    ]);

    let mut distribution = DistributionArray::default();
    distribution[0] = 0;
    distribution[1] = 0;

    println!("Prepare borrow authority");
    let (depositor_authority, _) = &everlend_utils::find_program_address(
        &everlend_depositor::id(),
        &initialiazed_accounts.depositor,
    );

    for key in required_mints {
        let mint = mint_map.get(key).unwrap();
        let collateral_mints = collateral_mint_map.get(key).unwrap();

        println!("General pool");
        let (general_pool_pubkey, general_pool_token_account, general_pool_mint) =
            general_pool::create_pool(config, &initialiazed_accounts.general_pool_market, mint)?;

        println!("Payer token account");
        let token_account = get_associated_token_address(&payer_pubkey, mint);
        println!("Payer pool account");
        // let pool_account = get_associated_token_address(&payer_pubkey, &general_pool_mint);
        let pool_account =
            spl_create_associated_token_account(config, &payer_pubkey, &general_pool_mint)?;

        println!("Income pool");
        let (income_pool_pubkey, income_pool_token_account) =
            income_pools::create_pool(config, &initialiazed_accounts.income_pool_market, mint)?;

        // MM Pools
        println!("MM Pool: Port Finance");
        let (
            port_finance_mm_pool_pubkey,
            port_finance_mm_pool_token_account,
            port_finance_mm_pool_mint,
        ) = ulp::create_pool(
            config,
            &initialiazed_accounts.mm_pool_markets[0],
            &collateral_mints[0],
        )?;

        println!("MM Pool: Larix");
        let (larix_mm_pool_pubkey, larix_mm_pool_token_account, larix_mm_pool_mint) =
            ulp::create_pool(
                config,
                &initialiazed_accounts.mm_pool_markets[1],
                &collateral_mints[1],
            )?;

        liquidity_oracle::create_token_distribution(
            config,
            &initialiazed_accounts.liquidity_oracle,
            mint,
            &distribution,
        )?;

        // Transit accounts
        let liquidity_transit_pubkey =
            depositor::create_transit(config, &initialiazed_accounts.depositor, mint, None)?;

        // Reserve
        println!("Reserve transit");
        let liquidity_reserve_transit_pubkey = depositor::create_transit(
            config,
            &initialiazed_accounts.depositor,
            mint,
            Some("reserve".to_string()),
        )?;
        // spl_token_transfer(
        //     config,
        //     &token_account,
        //     &liquidity_reserve_transit_pubkey,
        //     10000,
        // )?;

        depositor::create_transit(
            config,
            &initialiazed_accounts.depositor,
            &collateral_mints[0],
            None,
        )?;
        depositor::create_transit(
            config,
            &initialiazed_accounts.depositor,
            &collateral_mints[1],
            None,
        )?;

        depositor::create_transit(
            config,
            &initialiazed_accounts.depositor,
            &port_finance_mm_pool_mint,
            None,
        )?;
        depositor::create_transit(
            config,
            &initialiazed_accounts.depositor,
            &larix_mm_pool_mint,
            None,
        )?;

        // Borrow authorities
        general_pool::create_pool_borrow_authority(
            config,
            &initialiazed_accounts.general_pool_market,
            &general_pool_pubkey,
            depositor_authority,
            10_000, // 100%
        )?;

        initialiazed_accounts.token_accounts.insert(
            key.to_string(),
            TokenAccounts {
                mint: *mint,
                liquidity_token_account: token_account,
                collateral_token_account: pool_account,
                general_pool: general_pool_pubkey,
                general_pool_token_account,
                general_pool_mint,
                income_pool: income_pool_pubkey,
                income_pool_token_account,
                mm_pools: vec![
                    MoneyMarketAccounts {
                        pool: port_finance_mm_pool_pubkey,
                        pool_token_account: port_finance_mm_pool_token_account,
                        token_mint: collateral_mints[0],
                        pool_mint: port_finance_mm_pool_mint,
                    },
                    MoneyMarketAccounts {
                        pool: larix_mm_pool_pubkey,
                        pool_token_account: larix_mm_pool_token_account,
                        token_mint: collateral_mints[1],
                        pool_mint: larix_mm_pool_mint,
                    },
                ],
                liquidity_transit: liquidity_transit_pubkey,
            },
        );
    }

    initialiazed_accounts
        .save(&format!("accounts.{}.yaml", config.network))
        .unwrap();

    Ok(())
}

async fn command_add_reserve_liquidity(
    config: &Config,
    mint_key: &str,
    amount: u64,
) -> anyhow::Result<()> {
    let payer_pubkey = config.fee_payer.pubkey();
    let default_accounts = get_default_accounts(config);
    let initialiazed_accounts = get_initialized_accounts(config);

    let mint_map = HashMap::from([
        ("SOL".to_string(), default_accounts.sol_mint),
        ("USDC".to_string(), default_accounts.usdc_mint),
        ("USDT".to_string(), default_accounts.usdt_mint),
    ]);
    let mint = mint_map.get(mint_key).unwrap();

    let (liquidity_reserve_transit_pubkey, _) = everlend_depositor::find_transit_program_address(
        &everlend_depositor::id(),
        &initialiazed_accounts.depositor,
        mint,
        "reserve",
    );

    let token_account = get_associated_token_address(&payer_pubkey, mint);

    println!(
        "Transfer {} lamports of {} to reserve liquidity account",
        amount, mint_key
    );

    spl_token_transfer(
        config,
        &token_account,
        &liquidity_reserve_transit_pubkey,
        amount,
    )?;

    Ok(())
}

async fn command_create(
    config: &Config,
    accounts_path: &str,
    required_mints: Vec<&str>,
) -> anyhow::Result<()> {
    let payer_pubkey = config.fee_payer.pubkey();
    println!("Fee payer: {}", payer_pubkey);

    let default_accounts = get_default_accounts(config);

    let mint_map = HashMap::from([
        ("SOL".to_string(), default_accounts.sol_mint),
        ("USDC".to_string(), default_accounts.usdc_mint),
        ("USDT".to_string(), default_accounts.usdt_mint),
    ]);

    let collateral_mint_map = HashMap::from([
        ("SOL".to_string(), default_accounts.sol_collateral),
        ("USDC".to_string(), default_accounts.usdc_collateral),
        ("USDT".to_string(), default_accounts.usdt_collateral),
    ]);

    println!("Registry");
    let registry_pubkey = registry::init(config, None)?;
    let mut registry_config = SetRegistryConfigParams {
        general_pool_program_id: everlend_general_pool::id(),
        ulp_program_id: everlend_ulp::id(),
        liquidity_oracle_program_id: everlend_liquidity_oracle::id(),
        depositor_program_id: everlend_depositor::id(),
        income_pools_program_id: everlend_income_pools::id(),
        money_market_program_ids: [Pubkey::default(); TOTAL_DISTRIBUTIONS],
        refresh_income_interval: REFRESH_INCOME_INTERVAL,
    };
    registry_config.money_market_program_ids[0] = default_accounts.port_finance_program_id;
    registry_config.money_market_program_ids[1] = default_accounts.larix_program_id;
    registry_config.money_market_program_ids[2] = default_accounts.solend_program_id;

    println!("registry_config = {:#?}", registry_config);

    registry::set_registry_config(config, &registry_pubkey, registry_config)?;

    let general_pool_market_pubkey = general_pool::create_market(config, None)?;
    let income_pool_market_pubkey =
        income_pools::create_market(config, None, &general_pool_market_pubkey)?;

    let port_finance_mm_pool_market_pubkey = ulp::create_market(config, None)?;
    let larix_mm_pool_market_pubkey = ulp::create_market(config, None)?;

    println!("Liquidity oracle");
    let liquidity_oracle_pubkey = liquidity_oracle::init(config, None)?;
    let mut distribution = DistributionArray::default();
    distribution[0] = 0;
    distribution[1] = 0;

    println!("Depositor");
    let depositor_pubkey = depositor::init(
        config,
        &registry_pubkey,
        None,
        &general_pool_market_pubkey,
        &income_pool_market_pubkey,
        &liquidity_oracle_pubkey,
    )?;

    println!("Prepare borrow authority");
    let (depositor_authority, _) =
        &everlend_utils::find_program_address(&everlend_depositor::id(), &depositor_pubkey);

    let mut token_accounts = HashMap::new();

    for key in required_mints {
        let mint = mint_map.get(key).unwrap();
        let collateral_mints = collateral_mint_map.get(key).unwrap();

        let (general_pool_pubkey, general_pool_token_account, general_pool_mint) =
            general_pool::create_pool(config, &general_pool_market_pubkey, mint)?;

        let token_account = get_associated_token_address(&payer_pubkey, mint);
        let pool_account =
            spl_create_associated_token_account(config, &payer_pubkey, &general_pool_mint)?;

        let (income_pool_pubkey, income_pool_token_account) =
            income_pools::create_pool(config, &income_pool_market_pubkey, mint)?;

        // MM Pools
        println!("MM Pool: Port Finance");
        let (
            port_finance_mm_pool_pubkey,
            port_finance_mm_pool_token_account,
            port_finance_mm_pool_mint,
        ) = ulp::create_pool(
            config,
            &port_finance_mm_pool_market_pubkey,
            &collateral_mints[0],
        )?;

        println!("MM Pool: Larix");
        let (larix_mm_pool_pubkey, larix_mm_pool_token_account, larix_mm_pool_mint) =
            ulp::create_pool(config, &larix_mm_pool_market_pubkey, &collateral_mints[1])?;

        liquidity_oracle::create_token_distribution(
            config,
            &liquidity_oracle_pubkey,
            mint,
            &distribution,
        )?;

        // Transit accounts
        let liquidity_transit_pubkey =
            depositor::create_transit(config, &depositor_pubkey, mint, None)?;

        // Reserve
        println!("Reserve transit");
        let liquidity_reserve_transit_pubkey = depositor::create_transit(
            config,
            &depositor_pubkey,
            mint,
            Some("reserve".to_string()),
        )?;
        spl_token_transfer(
            config,
            &token_account,
            &liquidity_reserve_transit_pubkey,
            10000,
        )?;

        depositor::create_transit(config, &depositor_pubkey, &collateral_mints[0], None)?;
        depositor::create_transit(config, &depositor_pubkey, &collateral_mints[1], None)?;

        depositor::create_transit(config, &depositor_pubkey, &port_finance_mm_pool_mint, None)?;
        depositor::create_transit(config, &depositor_pubkey, &larix_mm_pool_mint, None)?;

        // Borrow authorities
        general_pool::create_pool_borrow_authority(
            config,
            &general_pool_market_pubkey,
            &general_pool_pubkey,
            depositor_authority,
            10_000, // 100%
        )?;

        token_accounts.insert(
            key.to_string(),
            TokenAccounts {
                mint: *mint,
                liquidity_token_account: token_account,
                collateral_token_account: pool_account,
                general_pool: general_pool_pubkey,
                general_pool_token_account,
                general_pool_mint,
                income_pool: income_pool_pubkey,
                income_pool_token_account,
                mm_pools: vec![
                    MoneyMarketAccounts {
                        pool: port_finance_mm_pool_pubkey,
                        pool_token_account: port_finance_mm_pool_token_account,
                        token_mint: collateral_mints[0],
                        pool_mint: port_finance_mm_pool_mint,
                    },
                    MoneyMarketAccounts {
                        pool: larix_mm_pool_pubkey,
                        pool_token_account: larix_mm_pool_token_account,
                        token_mint: collateral_mints[1],
                        pool_mint: larix_mm_pool_mint,
                    },
                ],
                liquidity_transit: liquidity_transit_pubkey,
            },
        );
    }

    let initialiazed_accounts = InitializedAccounts {
        payer: payer_pubkey,
        registry: registry_pubkey,
        general_pool_market: general_pool_market_pubkey,
        income_pool_market: income_pool_market_pubkey,
        mm_pool_markets: vec![
            port_finance_mm_pool_market_pubkey,
            larix_mm_pool_market_pubkey,
        ],
        token_accounts,
        liquidity_oracle: liquidity_oracle_pubkey,
        depositor: depositor_pubkey,
    };

    initialiazed_accounts.save(accounts_path).unwrap();

    Ok(())
}

async fn command_info(config: &Config, accounts_path: &str) -> anyhow::Result<()> {
    let initialiazed_accounts = InitializedAccounts::load(accounts_path).unwrap_or_default();
    let default_accounts = get_default_accounts(config);

    println!("fee_payer: {:?}", config.fee_payer.pubkey());
    println!("default_accounts = {:#?}", default_accounts);
    println!("{:#?}", initialiazed_accounts);

    println!(
        "{:#?}",
        get_general_pool_market(config, &initialiazed_accounts.general_pool_market)?
    );

    for (_, token_accounts) in initialiazed_accounts.token_accounts {
        println!("mint = {:?}", token_accounts.mint);
        let (withdraw_requests_pubkey, withdraw_requests) = get_withdrawal_requests(
            config,
            &initialiazed_accounts.general_pool_market,
            &token_accounts.mint,
        )?;
        println!("{:#?}", (withdraw_requests_pubkey, &withdraw_requests));
    }

    Ok(())
}

async fn command_run_test(
    config: &Config,
    accounts_path: &str,
    case: Option<String>,
) -> anyhow::Result<()> {
    println!("Run {:?}", case);

    let default_accounts = get_default_accounts(config);
    let initialiazed_accounts = InitializedAccounts::load(accounts_path).unwrap_or_default();
    println!("default_accounts = {:#?}", default_accounts);

    let InitializedAccounts {
        payer,
        registry,
        general_pool_market,
        income_pool_market,
        mm_pool_markets,
        token_accounts,
        liquidity_oracle,
        depositor,
    } = initialiazed_accounts;

    let (registry_config_pubkey, _) =
        find_config_program_address(&everlend_registry::id(), &registry);

    println!("registry_pubkey = {:#?}", registry);
    println!("registry_config_pubkey = {:#?}", registry_config_pubkey);

    let registry_config_account = config.rpc_client.get_account(&registry_config_pubkey)?;
    let registry_config = RegistryConfig::unpack(&registry_config_account.data).unwrap();

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
        reserve_liquidity_supply: default_accounts.solend_reserve_sol_supply,
        reserve_liquidity_pyth_oracle: default_accounts.solend_reserve_pyth_oracle,
        reserve_liquidity_switchboard_oracle: default_accounts.solend_reserve_switchboard_oracle,
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
            1 => MoneyMarketPubkeys::Larix(larix_pubkeys.clone()),
            2 => MoneyMarketPubkeys::SPL(port_finance_pubkeys.clone()),
            _ => MoneyMarketPubkeys::Solend(solend_pubkeys.clone()),
        };

        depositor::deposit(
            config,
            &registry,
            &depositor,
            &mm_pool_markets[i],
            &sol.mm_pools[i].pool_token_account,
            &sol.mint,
            &sol.mm_pools[i].token_mint,
            &sol.mm_pools[i].pool_mint,
            &registry_config.money_market_program_ids[i],
            integrations::deposit_accounts(&registry_config.money_market_program_ids[i], &pubkeys),
        )
    };

    let withdraw = |i| {
        println!("Rebalancing: Withdraw: {}", i);
        let pubkeys = match i {
            1 => MoneyMarketPubkeys::Larix(larix_pubkeys.clone()),
            2 => MoneyMarketPubkeys::SPL(port_finance_pubkeys.clone()),
            _ => MoneyMarketPubkeys::Solend(solend_pubkeys.clone()),
        };

        depositor::withdraw(
            config,
            &registry,
            &depositor,
            &income_pool_market,
            &sol.income_pool_token_account,
            &mm_pool_markets[i],
            &sol.mm_pools[i].pool_token_account,
            &sol.mm_pools[i].token_mint,
            &sol.mint,
            &sol.mm_pools[i].pool_mint,
            &registry_config.money_market_program_ids[i],
            integrations::withdraw_accounts(&registry_config.money_market_program_ids[i], &pubkeys),
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

            update_token_distribution(distribution!([0, 100000000, 900000000]))?;
            let (_, rebalancing) = start_rebalancing()?;
            complete_rebalancing(Some(rebalancing))?;
        }
        Some("larix") => {
            general_pool_deposit(1000)?;

            update_token_distribution(distribution!([0, 1000000000]))?;
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let matches = App::new(crate_name!())
        .about(crate_description!())
        .version(crate_version!())
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .arg({
            let arg = Arg::with_name("config_file")
                .short("C")
                .long("config")
                .value_name("PATH")
                .takes_value(true)
                .global(true)
                .help("Configuration file to use");
            if let Some(ref config_file) = *solana_cli_config::CONFIG_FILE {
                arg.default_value(config_file)
            } else {
                arg
            }
        })
        .arg(
            Arg::with_name("verbose")
                .short("v")
                .long("verbose")
                .takes_value(false)
                .global(true)
                .help("Show additional information"),
        )
        .arg(
            Arg::with_name("owner")
                .long("owner")
                .value_name("KEYPAIR")
                .validator(is_keypair)
                .takes_value(true)
                .global(true)
                .help(
                    "Specify the token owner account. \
                     This may be a keypair file, the ASK keyword. \
                     Defaults to the client keypair.",
                ),
        )
        .arg(fee_payer_arg().global(true))
        .subcommand(
            SubCommand::with_name("create-registry")
                .about("Create a new registry")
                .arg(
                    Arg::with_name("keypair")
                        .long("keypair")
                        .validator(is_keypair)
                        .value_name("KEYPAIR")
                        .takes_value(true)
                        .help("Keypair [default: new keypair]"),
                ),
        )
        .subcommand(
            SubCommand::with_name("create-general-pool-market")
                .about("Create a new general pool market")
                .arg(
                    Arg::with_name("keypair")
                        .long("keypair")
                        .validator(is_keypair)
                        .value_name("KEYPAIR")
                        .takes_value(true)
                        .help("Keypair [default: new keypair]"),
                ),
        )
        .subcommand(
            SubCommand::with_name("create-income-pool-market")
                .about("Create a new income pool market")
                .arg(
                    Arg::with_name("keypair")
                        .validator(is_keypair)
                        .long("keypair")
                        .value_name("KEYPAIR")
                        .takes_value(true)
                        .help("Keypair [default: new keypair]"),
                ),
        )
        .subcommand(
            SubCommand::with_name("create-mm-pool-market")
                .about("Create a new MM pool market")
                .arg(
                    Arg::with_name("money-market")
                        .short("mm")
                        .long("money-market")
                        .value_name("NUMBER")
                        .takes_value(true)
                        .required(true)
                        .help("Money market index"),
                )
                .arg(
                    Arg::with_name("keypair")
                        .long("keypair")
                        .validator(is_keypair)
                        .value_name("KEYPAIR")
                        .takes_value(true)
                        .help("Keypair [default: new keypair]"),
                ),
        )
        .subcommand(
            SubCommand::with_name("create-liquidity-oracle")
                .about("Create a new liquidity oracle")
                .arg(
                    Arg::with_name("keypair")
                        .long("keypair")
                        .validator(is_keypair)
                        .value_name("KEYPAIR")
                        .takes_value(true)
                        .help("Keypair [default: new keypair]"),
                ),
        )
        .subcommand(
            SubCommand::with_name("create-depositor")
                .about("Create a new depositor")
                .arg(
                    Arg::with_name("keypair")
                        .long("keypair")
                        .validator(is_keypair)
                        .value_name("KEYPAIR")
                        .takes_value(true)
                        .help("Keypair [default: new keypair]"),
                ),
        )
        .subcommand(
            SubCommand::with_name("create-token-accounts")
                .about("Create a new token accounts")
                .arg(
                    Arg::with_name("mints")
                        .multiple(true)
                        .long("mints")
                        .short("m")
                        .required(true)
                        .min_values(1)
                        .takes_value(true),
                ),
        )
        .subcommand(
            SubCommand::with_name("add-reserve-liquidity")
                .about("Transfer liquidity to reserve account")
                .arg(
                    Arg::with_name("mint")
                        .long("mint")
                        .short("m")
                        .required(true)
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("amount")
                        .long("amount")
                        .validator(is_amount)
                        .value_name("NUMBER")
                        .takes_value(true)
                        .required(true)
                        .help("Liquidity amount"),
                ),
        )
        .subcommand(
            SubCommand::with_name("create")
                .about("Create a new accounts")
                .arg(
                    Arg::with_name("mints")
                        .multiple(true)
                        .long("mints")
                        .short("m")
                        .required(true)
                        .min_values(1)
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("accounts")
                        .short("A")
                        .long("accounts")
                        .value_name("PATH")
                        .takes_value(true)
                        .help("Accounts file"),
                ),
        )
        .subcommand(
            SubCommand::with_name("info")
                .about("Print out env information")
                .arg(
                    Arg::with_name("accounts")
                        .short("A")
                        .long("accounts")
                        .value_name("PATH")
                        .takes_value(true)
                        .help("Accounts file"),
                ),
        )
        .subcommand(
            SubCommand::with_name("test")
                .about("Run a test")
                .arg(
                    Arg::with_name("case")
                        .value_name("NAME")
                        .takes_value(true)
                        .index(1)
                        .help("Case"),
                )
                .arg(
                    Arg::with_name("accounts")
                        .short("A")
                        .long("accounts")
                        .value_name("PATH")
                        .takes_value(true)
                        .help("Accounts file"),
                ),
        )
        .get_matches();

    let mut wallet_manager = None;
    let config = {
        let cli_config = if let Some(config_file) = matches.value_of("config_file") {
            println!("config_file = {:?}", config_file);
            solana_cli_config::Config::load(config_file).unwrap_or_default()
        } else {
            solana_cli_config::Config::default()
        };

        let json_rpc_url = value_t!(matches, "json_rpc_url", String)
            .unwrap_or_else(|_| cli_config.json_rpc_url.clone());
        let network = url_to_moniker(&json_rpc_url);
        println!("network = {:?}", network);

        let owner = signer_from_path(
            &matches,
            matches
                .value_of("owner")
                .unwrap_or(&cli_config.keypair_path),
            "owner",
            &mut wallet_manager,
        )
        .unwrap_or_else(|err| {
            eprintln!("Error: {}", err);
            exit(1);
        });

        let fee_payer = signer_from_path(
            &matches,
            matches
                .value_of("fee_payer")
                .unwrap_or(&cli_config.keypair_path),
            "fee_payer",
            &mut wallet_manager,
        )
        .unwrap_or_else(|err| {
            eprintln!("Error: {}", err);
            exit(1);
        });

        let verbose = matches.is_present("verbose");

        Config {
            rpc_client: RpcClient::new_with_commitment(json_rpc_url, CommitmentConfig::confirmed()),
            verbose,
            owner,
            fee_payer,
            network,
        }
    };

    solana_logger::setup_with_default("solana=info");

    let _ = match matches.subcommand() {
        ("create-registry", Some(arg_matches)) => {
            let keypair = keypair_of(arg_matches, "keypair");
            command_create_registry(&config, keypair).await
        }
        ("create-general-pool-market", Some(arg_matches)) => {
            let keypair = keypair_of(arg_matches, "keypair");
            command_create_general_pool_market(&config, keypair).await
        }
        ("create-income-pool-market", Some(arg_matches)) => {
            let keypair = keypair_of(arg_matches, "keypair");
            command_create_income_pool_market(&config, keypair).await
        }
        ("create-mm-pool-market", Some(arg_matches)) => {
            let keypair = keypair_of(arg_matches, "keypair");
            let money_market = value_of::<usize>(arg_matches, "money-market").unwrap();
            command_create_mm_pool_market(&config, keypair, MoneyMarket::from(money_market)).await
        }
        ("create-liquidity-oracle", Some(arg_matches)) => {
            let keypair = keypair_of(arg_matches, "keypair");
            command_create_liquidity_oracle(&config, keypair).await
        }
        ("create-depositor", Some(arg_matches)) => {
            let keypair = keypair_of(arg_matches, "keypair");
            command_create_depositor(&config, keypair).await
        }
        ("create-token-accounts", Some(arg_matches)) => {
            let mints: Vec<_> = arg_matches.values_of("mints").unwrap().collect();
            command_create_token_accounts(&config, mints).await
        }
        ("add-reserve-liquidity", Some(arg_matches)) => {
            let mint = arg_matches.value_of("mint").unwrap();
            let amount = value_of::<u64>(arg_matches, "amount").unwrap();
            command_add_reserve_liquidity(&config, mint, amount).await
        }
        ("create", Some(arg_matches)) => {
            let accounts_path = arg_matches.value_of("accounts").unwrap_or("accounts.yaml");
            let mints: Vec<_> = arg_matches.values_of("mints").unwrap().collect();
            command_create(&config, accounts_path, mints).await
        }
        ("info", Some(arg_matches)) => {
            let accounts_path = arg_matches.value_of("accounts").unwrap_or("accounts.yaml");
            command_info(&config, accounts_path).await
        }
        ("test", Some(arg_matches)) => {
            let accounts_path = arg_matches.value_of("accounts").unwrap_or("accounts.yaml");
            let case = value_of::<String>(arg_matches, "case");
            command_run_test(&config, accounts_path, case).await
        }
        _ => unreachable!(),
    }
    .map_err(|err| {
        eprintln!("{}", err);
        exit(1);
    });

    Ok(())
}
