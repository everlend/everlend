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
use everlend_liquidity_oracle::state::DistributionArray;
use everlend_registry::{
    find_config_program_address,
    state::{RegistryConfig, SetRegistryConfigParams, TOTAL_DISTRIBUTIONS},
};
use everlend_utils::integrations::{self, MoneyMarketPubkeys};
use solana_clap_utils::{
    fee_payer::fee_payer_arg,
    input_parsers::value_of,
    input_validators::{is_keypair, is_url_or_moniker},
    keypair::signer_from_path,
};
use solana_client::rpc_client::RpcClient;
use solana_program::{program_pack::Pack, pubkey::Pubkey};
use solana_sdk::commitment_config::CommitmentConfig;
use spl_associated_token_account::get_associated_token_address;
use std::{collections::HashMap, process::exit, str::FromStr};
use utils::*;

/// Generates fixed distribution from slice
#[macro_export]
macro_rules! distribution {
    ($distribuition:expr) => {{
        let mut new_distribuition = DistributionArray::default();
        new_distribuition[..$distribuition.len()].copy_from_slice(&$distribuition);
        new_distribuition
    }};
}

async fn command_create(
    config: &Config,
    accounts_path: &str,
    required_mints: Vec<&str>,
) -> anyhow::Result<()> {
    let payer_pubkey = config.fee_payer.pubkey();
    println!("Fee payer: {}", payer_pubkey);

    let mint_map = HashMap::from([
        ("SOL".to_string(), Pubkey::from_str(SOL_MINT).unwrap()),
        ("USDC".to_string(), Pubkey::from_str(USDC_MINT).unwrap()),
        ("USDT".to_string(), Pubkey::from_str(USDT_MINT).unwrap()),
    ]);

    let collateral_mint_map = HashMap::from([
        (
            "SOL".to_string(),
            [
                Pubkey::from_str(PORT_FINANCE_RESERVE_SOL_COLLATERAL_MINT).unwrap(),
                Pubkey::from_str(LARIX_RESERVE_SOL_COLLATERAL_MINT).unwrap(),
            ],
        ),
        (
            "USDC".to_string(),
            [
                Pubkey::from_str(PORT_FINANCE_RESERVE_USDC_COLLATERAL_MINT).unwrap(),
                Pubkey::from_str(LARIX_RESERVE_USDC_COLLATERAL_MINT).unwrap(),
            ],
        ),
        (
            "USDT".to_string(),
            [
                Pubkey::from_str(PORT_FINANCE_RESERVE_USDT_COLLATERAL_MINT).unwrap(),
                Pubkey::from_str(LARIX_RESERVE_USDT_COLLATERAL_MINT).unwrap(),
            ],
        ),
    ]);

    let port_finance_program_id = Pubkey::from_str(integrations::PORT_FINANCE_PROGRAM_ID).unwrap();
    let larix_program_id = Pubkey::from_str(integrations::LARIX_PROGRAM_ID).unwrap();

    println!("Registry");
    let registry_pubkey = registry::init(config, None)?;
    let mut registry_config = SetRegistryConfigParams {
        general_pool_program_id: everlend_general_pool::id(),
        ulp_program_id: everlend_ulp::id(),
        liquidity_oracle_program_id: everlend_liquidity_oracle::id(),
        depositor_program_id: everlend_depositor::id(),
        income_pools_program_id: everlend_income_pools::id(),
        money_market_program_ids: [Pubkey::default(); TOTAL_DISTRIBUTIONS],
    };
    registry_config.money_market_program_ids[0] = port_finance_program_id;
    registry_config.money_market_program_ids[1] = larix_program_id;

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
        let liquidity_transit_pubkey = depositor::create_transit(config, &depositor_pubkey, mint)?;

        depositor::create_transit(config, &depositor_pubkey, &collateral_mints[0])?;
        depositor::create_transit(config, &depositor_pubkey, &collateral_mints[1])?;

        depositor::create_transit(config, &depositor_pubkey, &port_finance_mm_pool_mint)?;
        depositor::create_transit(config, &depositor_pubkey, &larix_mm_pool_mint)?;

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

    println!("{:#?}", initialiazed_accounts);

    Ok(())
}

async fn command_run_test(
    config: &Config,
    accounts_path: &str,
    case: Option<String>,
) -> anyhow::Result<()> {
    println!("Run {:?}", case);

    let initialiazed_accounts = InitializedAccounts::load(accounts_path).unwrap_or_default();

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
    let registry_config_account = config.rpc_client.get_account(&registry_config_pubkey)?;
    let registry_config = RegistryConfig::unpack(&registry_config_account.data).unwrap();

    let sol = token_accounts.get("SOL").unwrap();

    let sol_oracle = Pubkey::from_str(SOL_ORACLE).unwrap();
    let port_finance_pubkeys = integrations::spl_token_lending::AccountPubkeys {
        reserve: Pubkey::from_str(PORT_FINANCE_RESERVE_SOL).unwrap(),
        reserve_liquidity_supply: Pubkey::from_str(PORT_FINANCE_RESERVE_SOL_SUPPLY).unwrap(),
        reserve_liquidity_oracle: sol_oracle,
        lending_market: Pubkey::from_str(PORT_FINANCE_LENDING_MARKET).unwrap(),
    };
    let larix_pubkeys = integrations::larix::AccountPubkeys {
        reserve: Pubkey::from_str(LARIX_RESERVE_SOL).unwrap(),
        reserve_liquidity_supply: Pubkey::from_str(LARIX_RESERVE_SOL_SUPPLY).unwrap(),
        reserve_liquidity_oracle: sol_oracle,
        lending_market: Pubkey::from_str(LARIX_LENDING_MARKET).unwrap(),
    };

    let update_token_distribution = |d: DistributionArray| {
        liquidity_oracle::update_token_distribution(config, &liquidity_oracle, &sol.mint, &d)
    };

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
        )
    };

    let deposit = |i: usize| {
        println!("Rebalancing: Deposit: {}", i);
        let pubkeys = match i {
            1 => MoneyMarketPubkeys::Larix(larix_pubkeys.clone()),
            _ => MoneyMarketPubkeys::SPL(port_finance_pubkeys.clone()),
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
            _ => MoneyMarketPubkeys::SPL(port_finance_pubkeys.clone()),
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

    let get_balance = |pk: &Pubkey| -> anyhow::Result<String> {
        Ok(config.rpc_client.get_token_account_balance(pk)?.amount)
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

    let general_pool_withdraw_request = |a: u64, i: u64| {
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
            i,
        )
    };

    let general_pool_withdraw = |i: u64| {
        println!("Withdraw");
        general_pool::withdraw(
            config,
            &general_pool_market,
            &sol.general_pool,
            &sol.liquidity_token_account,
            &sol.general_pool_token_account,
            &sol.mint,
            &sol.general_pool_mint,
            i,
        )
    };

    match case.as_deref() {
        Some("first") => {
            general_pool_deposit(1000)?;
            update_token_distribution(distribution!([1000000000, 0]))?;
            let (_, rebalancing) = start_rebalancing()?;
            println!("{:#?}", rebalancing);
            deposit(0)?;
            println!("b = {:?}", get_balance(&sol.liquidity_transit)?);

            update_token_distribution(distribution!([959876767, 0]))?;
            let (_, rebalancing) = start_rebalancing()?;
            println!("{:#?}", rebalancing);
            println!("b = {:?}", get_balance(&sol.liquidity_transit)?);
            withdraw(0)?;
            println!("b = {:?}", get_balance(&sol.liquidity_transit)?);

            update_token_distribution(distribution!([959876767, 0]))?;
            let (_, rebalancing) = start_rebalancing()?;
            println!("{:#?}", rebalancing);
            println!("b = {:?}", get_balance(&sol.liquidity_transit)?);
            println!("b(g) = {:?}", get_balance(&sol.general_pool_token_account)?);
            withdraw(0)?;

            update_token_distribution(distribution!([959876767, 0]))?;
            let (_, rebalancing) = start_rebalancing()?;
            println!("{:#?}", rebalancing);
            println!("b = {:?}", get_balance(&sol.liquidity_transit)?);
        }
        Some("second") => {
            general_pool_deposit(1000)?;
            update_token_distribution(distribution!([500000000, 500000000]))?;
            let (_, rebalancing) = start_rebalancing()?;
            println!("{:#?}", rebalancing);
            deposit(0)?;
            deposit(1)?;
            println!("b = {:?}", get_balance(&sol.liquidity_transit)?);

            general_pool_deposit(10)?;

            update_token_distribution(distribution!([900000000, 100000000]))?;
            let (_, rebalancing) = start_rebalancing()?;
            println!("{:#?}", rebalancing);
            println!("b = {:?}", get_balance(&sol.liquidity_transit)?);
        }
        Some("larix") => {
            general_pool_deposit(1000)?;
            update_token_distribution(distribution!([0, 1000000000]))?;
            let (_, rebalancing) = start_rebalancing()?;
            println!("{:#?}", rebalancing);
            deposit(1)?;
        }
        Some("zero-distribution") => {
            general_pool_deposit(1000)?;
            update_token_distribution(distribution!([0, 0]))?;
            let (_, rebalancing) = start_rebalancing()?;
            println!("{:#?}", rebalancing);
        }
        Some("deposit") => {
            general_pool_deposit(1000)?;
            update_token_distribution(distribution!([1000000000, 0]))?;
            let (_, rebalancing) = start_rebalancing()?;
            println!("{:#?}", rebalancing);
            deposit(0)?;

            general_pool_deposit(1000)?;

            update_token_distribution(distribution!([1000000000, 0]))?;
            let (_, rebalancing) = start_rebalancing()?;
            println!("{:#?}", rebalancing);
        }
        Some("full") => {
            general_pool_deposit(1000)?;
            update_token_distribution(distribution!([1000000000, 0]))?;
            let (_, rebalancing) = start_rebalancing()?;
            println!("{:#?}", rebalancing);
            deposit(0)?;

            general_pool_withdraw_request(100, 1)?;

            update_token_distribution(distribution!([1000000000, 0]))?;
            let (_, rebalancing) = start_rebalancing()?;
            println!("{:#?}", rebalancing);
            withdraw(0)?;

            update_token_distribution(distribution!([1000000000, 0]))?;
            let (_, rebalancing) = start_rebalancing()?;
            println!("{:#?}", rebalancing);
            general_pool_withdraw(1)?;

            update_token_distribution(distribution!([1000000000, 0]))?;
            let (_, rebalancing) = start_rebalancing()?;
            println!("{:#?}", rebalancing);
        }
        Some("full-two-withdrawal-requests") => {
            general_pool_deposit(1000)?;
            update_token_distribution(distribution!([1000000000, 0]))?;
            let (_, rebalancing) = start_rebalancing()?;
            println!("{:#?}", rebalancing);
            deposit(0)?;

            general_pool_withdraw_request(100, 1)?;

            update_token_distribution(distribution!([1000000000, 0]))?;
            let (_, rebalancing) = start_rebalancing()?;
            println!("{:#?}", rebalancing);
            withdraw(0)?;

            general_pool_withdraw_request(50, 2)?;

            update_token_distribution(distribution!([1000000000, 0]))?;
            let (_, rebalancing) = start_rebalancing()?;
            println!("{:#?}", rebalancing);
            general_pool_withdraw(1)?;
            withdraw(0)?;

            update_token_distribution(distribution!([1000000000, 0]))?;
            let (_, rebalancing) = start_rebalancing()?;
            println!("{:#?}", rebalancing);
            general_pool_withdraw(2)?;
        }
        None => {
            general_pool_deposit(1000)?;
            update_token_distribution(distribution!([500000000, 500000000]))?;
            start_rebalancing()?;
            deposit(0)?;
            deposit(1)?;
            println!("b = {:?}", get_balance(&sol.liquidity_transit)?);

            general_pool_withdraw_request(100, 1)?;

            update_token_distribution(distribution!([300000000, 600000000]))?;
            let (_, rebalancing) = start_rebalancing()?;
            println!("{:#?}", rebalancing);
            println!("b = {:?}", get_balance(&sol.liquidity_transit)?);
            withdraw(0)?;
            println!("b = {:?}", get_balance(&sol.liquidity_transit)?);
            deposit(1)?;

            update_token_distribution(distribution!([0, 1000000000]))?;
            let (_, rebalancing) = start_rebalancing()?;
            println!("{:#?}", rebalancing);
            general_pool_withdraw(1)?;
            withdraw(0)?;
            deposit(1)?;

            update_token_distribution(distribution!([100000000, 0]))?;
            let (_, rebalancing) = start_rebalancing()?;
            println!("{:#?}", rebalancing);
            withdraw(1)?;
            deposit(0)?;
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
            Arg::with_name("json_rpc_url")
                .short("u")
                .long("url")
                .value_name("URL_OR_MONIKER")
                .takes_value(true)
                .global(true)
                .validator(is_url_or_moniker)
                .help(
                    "URL for Solana's JSON RPC or moniker (or their first letter): \
                       [mainnet-beta, testnet, devnet, localhost] \
                    Default from the configuration file.",
                ),
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
            solana_cli_config::Config::load(config_file).unwrap_or_default()
        } else {
            solana_cli_config::Config::default()
        };

        let json_rpc_url = value_t!(matches, "json_rpc_url", String)
            .unwrap_or_else(|_| cli_config.json_rpc_url.clone());

        let owner = signer_from_path(
            &matches,
            &cli_config.keypair_path,
            "owner",
            &mut wallet_manager,
        )
        .unwrap_or_else(|err| {
            eprintln!("Error: {}", err);
            exit(1);
        });

        let fee_payer = signer_from_path(
            &matches,
            &cli_config.keypair_path,
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
        }
    };

    solana_logger::setup_with_default("solana=info");

    let _ = match matches.subcommand() {
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
