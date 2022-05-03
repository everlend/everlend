use std::collections::BTreeMap;
use std::convert::TryInto;
use std::{collections::HashMap, process::exit, str::FromStr};

use anyhow::bail;
use clap::{
    crate_description, crate_name, crate_version, value_t, App, AppSettings, Arg, SubCommand,
};
use regex::Regex;
use solana_clap_utils::input_parsers::pubkeys_of;
use solana_clap_utils::{
    fee_payer::fee_payer_arg,
    input_parsers::{keypair_of, pubkey_of, value_of},
    input_validators::{is_amount, is_keypair, is_pubkey},
    keypair::signer_from_path,
};
use solana_client::client_error::ClientError;
use solana_client::rpc_client::RpcClient;
use solana_program::pubkey::Pubkey;
use solana_sdk::commitment_config::CommitmentConfig;
use spl_associated_token_account::get_associated_token_address;

use accounts_config::*;
use commands::*;
use everlend_liquidity_oracle::state::DistributionArray;
use everlend_registry::state::{PoolMarketsConfig, SetRegistryConfigParams, TOTAL_DISTRIBUTIONS};
use everlend_utils::integrations::MoneyMarket;
use general_pool::get_withdrawal_requests;
use utils::*;

use crate::general_pool::get_general_pool_market;

mod accounts_config;
mod commands;
mod commands_multisig;
mod commands_test;
mod depositor;
mod general_pool;
mod income_pools;
mod liquidity_oracle;
mod multisig;
mod registry;
mod ulp;
mod utils;

pub fn url_to_moniker(url: &str) -> String {
    let re = Regex::new(r"devnet|mainnet|localhost|testnet").unwrap();
    let cap = &re.captures(url).unwrap()[0];

    match cap {
        "mainnet" => "mainnet-beta",
        _ => cap,
    }
    .to_string()
}

async fn command_create(
    config: &Config,
    accounts_path: &str,
    required_mints: Vec<&str>,
) -> anyhow::Result<()> {
    let payer_pubkey = config.fee_payer.pubkey();
    println!("Fee payer: {}", payer_pubkey);

    let default_accounts = config.get_default_accounts();

    let mint_map = HashMap::from([
        ("SOL".to_string(), default_accounts.sol_mint),
        ("USDC".to_string(), default_accounts.usdc_mint),
        ("USDT".to_string(), default_accounts.usdt_mint),
        ("mSOL".to_string(), default_accounts.msol_mint),
        ("stSOL".to_string(), default_accounts.stsol_mint),
        ("soBTC".to_string(), default_accounts.sobtc_mint),
        ("ETHw".to_string(), default_accounts.ethw_mint),
        ("USTw".to_string(), default_accounts.ustw_mint),
        ("FTTw".to_string(), default_accounts.fttw_mint),
        ("RAY".to_string(), default_accounts.ray_mint),
        ("SRM".to_string(), default_accounts.srm_mint),
    ]);

    let collateral_mint_map = HashMap::from([
        ("SOL".to_string(), default_accounts.sol_collateral),
        ("USDC".to_string(), default_accounts.usdc_collateral),
        ("USDT".to_string(), default_accounts.usdt_collateral),
        ("mSOL".to_string(), default_accounts.msol_collateral),
        ("stSOL".to_string(), default_accounts.stsol_collateral),
        ("soBTC".to_string(), default_accounts.sobtc_collateral),
        ("ETHw".to_string(), default_accounts.ethw_collateral),
        ("USTw".to_string(), default_accounts.ustw_collateral),
        ("FTTw".to_string(), default_accounts.fttw_collateral),
        ("RAY".to_string(), default_accounts.ray_collateral),
        ("SRM".to_string(), default_accounts.srm_collateral),
    ]);

    let general_pool_market_pubkey = general_pool::create_market(config, None)?;
    let income_pool_market_pubkey =
        income_pools::create_market(config, None, &general_pool_market_pubkey)?;

    let mm_pool_markets = vec![
        ulp::create_market(config, None)?,
        ulp::create_market(config, None)?,
        ulp::create_market(config, None)?,
    ];

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

    let mut ulp_pool_markets: [Pubkey; TOTAL_DISTRIBUTIONS] = Default::default();
    ulp_pool_markets[..mm_pool_markets.len()].copy_from_slice(&mm_pool_markets);

    let pool_markets_cfg = PoolMarketsConfig {
        general_pool_market: general_pool_market_pubkey,
        income_pool_market: income_pool_market_pubkey,
        ulp_pool_markets,
    };

    println!("registry_config = {:#?}", registry_config);
    println!("pool_markets_cfg = {:#?}", pool_markets_cfg);

    registry::set_registry_config(config, &registry_pubkey, registry_config, pool_markets_cfg)?;

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

    let mut token_accounts = BTreeMap::new();

    for key in required_mints {
        let mint = mint_map.get(key).unwrap();
        let collateral_mints: Vec<(Pubkey, Pubkey)> = collateral_mint_map
            .get(key)
            .unwrap()
            .iter()
            .zip(mm_pool_markets.iter())
            .filter_map(|(collateral_mint, mm_pool_market_pubkey)| {
                collateral_mint.map(|coll_mint| (coll_mint, *mm_pool_market_pubkey))
            })
            .collect();

        let (general_pool_pubkey, general_pool_token_account, general_pool_mint) =
            general_pool::create_pool(config, &general_pool_market_pubkey, mint)?;

        let token_account = get_associated_token_address(&payer_pubkey, mint);
        let pool_account =
            spl_create_associated_token_account(config, &payer_pubkey, &general_pool_mint)?;

        let (income_pool_pubkey, income_pool_token_account) =
            income_pools::create_pool(config, &income_pool_market_pubkey, mint)?;

        // MM Pools
        let mm_pool_pubkeys = collateral_mints
            .iter()
            .map(|(collateral_mint, mm_pool_market_pubkey)| {
                println!("MM Pool: {}", collateral_mint);
                ulp::create_pool(config, mm_pool_market_pubkey, collateral_mint)
            })
            .collect::<Result<Vec<(Pubkey, Pubkey, Pubkey)>, ClientError>>()?;

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

        collateral_mints
            .iter()
            .map(|(collateral_mint, _mm_pool_market_pubkey)| {
                depositor::create_transit(config, &depositor_pubkey, collateral_mint, None)
            })
            .collect::<Result<Vec<Pubkey>, ClientError>>()?;

        mm_pool_pubkeys
            .iter()
            .map(|(_, _, mm_pool_miny)| {
                depositor::create_transit(config, &depositor_pubkey, mm_pool_miny, None)
            })
            .collect::<Result<Vec<Pubkey>, ClientError>>()?;

        let mm_pools = collateral_mints
            .iter()
            .zip(mm_pool_pubkeys)
            .map(
                |(
                    (collateral_mint, _mm_pool_market_pubkey),
                    (mm_pool_pubkey, mm_pool_token_account, mm_pool_mint),
                )| {
                    MoneyMarketAccounts {
                        pool: mm_pool_pubkey,
                        pool_token_account: mm_pool_token_account,
                        token_mint: *collateral_mint,
                        pool_mint: mm_pool_mint,
                    }
                },
            )
            .collect();

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
                mm_pools,
                liquidity_transit: liquidity_transit_pubkey,
            },
        );
    }

    let initialized_accounts = InitializedAccounts {
        payer: payer_pubkey,
        registry: registry_pubkey,
        general_pool_market: general_pool_market_pubkey,
        income_pool_market: income_pool_market_pubkey,
        mm_pool_markets,
        token_accounts,
        liquidity_oracle: liquidity_oracle_pubkey,
        depositor: depositor_pubkey,
    };

    initialized_accounts.save(accounts_path).unwrap();

    Ok(())
}

async fn command_info(config: &Config, accounts_path: &str) -> anyhow::Result<()> {
    let initialiazed_accounts = InitializedAccounts::load(accounts_path).unwrap_or_default();
    let default_accounts = config.get_default_accounts();

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

async fn command_run_migrate(
    config: &Config,
    accounts_path: &str,
    case: Option<String>,
) -> anyhow::Result<()> {
    let initialiazed_accounts = InitializedAccounts::load(accounts_path).unwrap_or_default();

    if case.is_none() {
        println!("Migrate token mint not presented");
        return Ok(());
    }

    let token = initialiazed_accounts
        .token_accounts
        .get(&case.unwrap())
        .unwrap();

    println!("Migrate withdraw requests");
    general_pool::migrate_general_pool_account(config)?;
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
                )
                .arg(
                    Arg::with_name("general-pool-market")
                        .short("gpm")
                        .required(true)
                        .validator(is_pubkey)
                        .value_name("PUBKEY")
                        .takes_value(true)
                        .help("General pool market pubkey"),
                )
                .arg(
                    Arg::with_name("income-pool-market")
                        .short("ipm")
                        .required(true)
                        .validator(is_pubkey)
                        .value_name("PUBKEY")
                        .takes_value(true)
                        .help("Income pool market pubkey"),
                )
                .arg(
                    Arg::with_name("ulp-pool-markets")
                        .short("upm")
                        .required(true)
                        .multiple(true)
                        .min_values(1)
                        .max_values(TOTAL_DISTRIBUTIONS as u64)
                        .validator(is_pubkey)
                        .value_name("PUBKEY")
                        .takes_value(true)
                        .help("ULP pool market pubkey"),
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
        .subcommand(
            SubCommand::with_name("multisig")
                .about("Multisig")
                .subcommand(
                    SubCommand::with_name("create")
                        .about("Create a new multisig")
                        .arg(
                            Arg::with_name("owners")
                                .multiple(true)
                                .long("owners")
                                .required(true)
                                .min_values(1)
                                .takes_value(true),
                        )
                        .arg(
                            Arg::with_name("threshold")
                                .short("th")
                                .long("threshold")
                                .value_name("NUMBER")
                                .takes_value(true)
                                .required(true)
                                .help("Threshold"),
                        ),
                )
                .subcommand(
                    SubCommand::with_name("propose-upgrade")
                        .about("Propose program upgrade")
                        .arg(
                            Arg::with_name("program")
                                .long("program")
                                .validator(is_pubkey)
                                .value_name("ADDRESS")
                                .takes_value(true)
                                .required(true)
                                .help("Program pubkey"),
                        )
                        .arg(
                            Arg::with_name("buffer")
                                .long("buffer")
                                .validator(is_pubkey)
                                .value_name("ADDRESS")
                                .takes_value(true)
                                .required(true)
                                .help("Buffer pubkey"),
                        )
                        .arg(
                            Arg::with_name("spill")
                                .long("spill")
                                .validator(is_pubkey)
                                .value_name("ADDRESS")
                                .takes_value(true)
                                .required(true)
                                .help("Spill pubkey"),
                        )
                        .arg(
                            Arg::with_name("multisig")
                                .long("multisig")
                                .validator(is_pubkey)
                                .value_name("ADDRESS")
                                .takes_value(true)
                                .required(true)
                                .help("Multisig pubkey"),
                        ),
                )
                .subcommand(
                    SubCommand::with_name("approve")
                        .about("Approve transaction")
                        .arg(
                            Arg::with_name("transaction")
                                .long("transaction")
                                .short("tx")
                                .validator(is_pubkey)
                                .value_name("ADDRESS")
                                .takes_value(true)
                                .required(true)
                                .help("Transaction account pubkey"),
                        )
                        .arg(
                            Arg::with_name("multisig")
                                .long("multisig")
                                .validator(is_pubkey)
                                .value_name("ADDRESS")
                                .takes_value(true)
                                .required(true)
                                .help("Multisig pubkey"),
                        ),
                )
                .subcommand(
                    SubCommand::with_name("execute")
                        .about("Execute transaction")
                        .arg(
                            Arg::with_name("transaction")
                                .long("transaction")
                                .validator(is_pubkey)
                                .value_name("ADDRESS")
                                .takes_value(true)
                                .required(true)
                                .help("Transaction account pubkey"),
                        )
                        .arg(
                            Arg::with_name("multisig")
                                .long("multisig")
                                .validator(is_pubkey)
                                .value_name("ADDRESS")
                                .takes_value(true)
                                .required(true)
                                .help("Multisig pubkey"),
                        ),
                )
                .subcommand(
                    SubCommand::with_name("info").about("Multisig info").arg(
                        Arg::with_name("multisig")
                            .validator(is_pubkey)
                            .value_name("ADDRESS")
                            .takes_value(true)
                            .required(true)
                            .help("Multisig pubkey"),
                    ),
                ),
        )
        .subcommand(
            SubCommand::with_name("migrate-general-pool")
                .about("Migrate general pool account")
                .arg(
                    Arg::with_name("case")
                        .value_name("TOKEN")
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
            let general_pool_market = pubkey_of(arg_matches, "general-pool-market").unwrap();
            let income_pool_market = pubkey_of(arg_matches, "income-pool-market").unwrap();
            let mut ulp_pool_markets = pubkeys_of(arg_matches, "ulp-pool-markets").unwrap();
            if ulp_pool_markets.len() > TOTAL_DISTRIBUTIONS {
                bail!(
                    "Length of ulp-pool-markets must be <= {}",
                    TOTAL_DISTRIBUTIONS
                )
            };
            ulp_pool_markets.extend_from_slice(
                vec![Pubkey::default(); TOTAL_DISTRIBUTIONS - ulp_pool_markets.len()].as_slice(),
            );

            let pool_market_cfg = PoolMarketsConfig {
                general_pool_market,
                income_pool_market,
                ulp_pool_markets: ulp_pool_markets.try_into().unwrap(),
            };
            command_create_registry(&config, keypair, pool_market_cfg).await
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
            commands_test::command_run_test(&config, accounts_path, case).await
        }
        ("multisig", Some(arg_matches)) => {
            let _ = match arg_matches.subcommand() {
                ("create", Some(arg_matches)) => {
                    let owners: Vec<_> = arg_matches
                        .values_of("owners")
                        .unwrap()
                        .map(|str| Pubkey::from_str(str).unwrap())
                        .collect();
                    let threshold = value_of::<u64>(arg_matches, "threshold").unwrap();

                    commands_multisig::command_create_multisig(&config, owners, threshold).await
                }
                ("propose-upgrade", Some(arg_matches)) => {
                    let program_pubkey = pubkey_of(arg_matches, "program").unwrap();
                    let buffer_pubkey = pubkey_of(arg_matches, "buffer").unwrap();
                    let spill_pubkey = pubkey_of(arg_matches, "spill").unwrap();
                    let multisig_pubkey = pubkey_of(arg_matches, "multisig").unwrap();

                    commands_multisig::command_propose_upgrade(
                        &config,
                        &program_pubkey,
                        &buffer_pubkey,
                        &spill_pubkey,
                        &multisig_pubkey,
                    )
                    .await
                }
                ("approve", Some(arg_matches)) => {
                    let transaction_pubkey = pubkey_of(arg_matches, "transaction").unwrap();
                    let multisig_pubkey = pubkey_of(arg_matches, "multisig").unwrap();

                    commands_multisig::command_approve(
                        &config,
                        &multisig_pubkey,
                        &transaction_pubkey,
                    )
                    .await
                }
                ("execute", Some(arg_matches)) => {
                    let transaction_pubkey = pubkey_of(arg_matches, "transaction").unwrap();
                    let multisig_pubkey = pubkey_of(arg_matches, "multisig").unwrap();

                    commands_multisig::command_execute_transaction(
                        &config,
                        &multisig_pubkey,
                        &transaction_pubkey,
                    )
                    .await
                }
                ("info", Some(arg_matches)) => {
                    let multisig_pubkey = pubkey_of(arg_matches, "multisig").unwrap();

                    commands_multisig::command_info_multisig(&config, &multisig_pubkey).await
                }
                _ => unreachable!(),
            }
            .map_err(|err| {
                eprintln!("{}", err);
                exit(1);
            });

            Ok(())
        }
        ("migrate-general-pool", Some(arg_matches)) => {
            let accounts_path = arg_matches.value_of("accounts").unwrap_or("accounts.yaml");
            let case = value_of::<String>(arg_matches, "case");
            command_run_migrate(&config, accounts_path, case).await
        }
        _ => unreachable!(),
    }
    .map_err(|err| {
        eprintln!("{}", err);
        exit(1);
    });

    Ok(())
}
