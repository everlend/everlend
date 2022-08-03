use std::collections::BTreeMap;
use std::path::PathBuf;
use std::{process::exit, str::FromStr};

use clap::{
    crate_description, crate_name, crate_version, value_t, App, AppSettings, Arg, SubCommand,
};
use commands_test::{command_test_larix_mining_raw, command_test_quarry_mining_raw};
use everlend_depositor::find_rebalancing_program_address;
use everlend_depositor::state::Rebalancing;
use everlend_utils::find_program_address;
use regex::Regex;
use solana_clap_utils::{
    fee_payer::fee_payer_arg,
    input_parsers::{keypair_of, pubkey_of, value_of, values_of},
    input_validators::{is_amount, is_keypair, is_pubkey},
    keypair::signer_from_path,
};
use solana_client::client_error::ClientError;
use solana_client::rpc_client::RpcClient;
use solana_program::pubkey::Pubkey;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::signature::Keypair;
use spl_associated_token_account::get_associated_token_address;

use accounts_config::*;
use commands::*;
use everlend_liquidity_oracle::state::DistributionArray;
use everlend_registry::state::{
    RegistryPrograms, RegistryRootAccounts, RegistrySettings, SetRegistryPoolConfigParams,
    TOTAL_DISTRIBUTIONS,
};
use everlend_utils::integrations::{MoneyMarket, StakingMoneyMarket};
use general_pool::get_withdrawal_requests;
use utils::*;

use crate::collateral_pool::PoolPubkeys;
use crate::general_pool::get_general_pool_market;

mod accounts_config;
mod collateral_pool;
mod commands;
mod commands_multisig;
mod commands_test;
mod depositor;
mod download_account;
mod general_pool;
mod income_pools;
mod liquidity_mining;
mod liquidity_oracle;
mod multisig;
mod registry;
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
    rebalance_executor: Pubkey,
) -> anyhow::Result<()> {
    let payer_pubkey = config.fee_payer.pubkey();
    println!("Fee payer: {}", payer_pubkey);

    let default_accounts = config.get_default_accounts();

    let (mint_map, collateral_mint_map) = get_asset_maps(default_accounts.clone());

    println!("Registry");
    let registry_pubkey = registry::init(config, None)?;
    let mut programs = RegistryPrograms {
        general_pool_program_id: everlend_general_pool::id(),
        collateral_pool_program_id: everlend_collateral_pool::id(),
        liquidity_oracle_program_id: everlend_liquidity_oracle::id(),
        depositor_program_id: everlend_depositor::id(),
        income_pools_program_id: everlend_income_pools::id(),
        money_market_program_ids: [Pubkey::default(); TOTAL_DISTRIBUTIONS],
    };
    programs.money_market_program_ids[0] = default_accounts.port_finance.program_id;
    programs.money_market_program_ids[1] = default_accounts.larix.program_id;
    programs.money_market_program_ids[2] = default_accounts.solend.program_id;

    registry::set_registry_config(
        config,
        &registry_pubkey,
        programs,
        RegistryRootAccounts::default(),
        RegistrySettings {
            refresh_income_interval: REFRESH_INCOME_INTERVAL,
        },
    )?;
    println!("programs = {:#?}", programs);

    let general_pool_market_pubkey = general_pool::create_market(config, None, &registry_pubkey)?;
    let income_pool_market_pubkey =
        income_pools::create_market(config, None, &general_pool_market_pubkey)?;

    let mm_collateral_pool_markets = vec![
        collateral_pool::create_market(config, None)?,
        collateral_pool::create_market(config, None)?,
        collateral_pool::create_market(config, None)?,
    ];

    println!("Liquidity oracle");
    let liquidity_oracle_pubkey = liquidity_oracle::init(config, None)?;
    let mut distribution = DistributionArray::default();
    distribution[0] = 0;
    distribution[1] = 0;
    distribution[2] = 0;

    println!("Registry");
    let registry_pubkey = registry::init(config, None)?;
    let mut programs = RegistryPrograms {
        general_pool_program_id: everlend_general_pool::id(),
        collateral_pool_program_id: everlend_collateral_pool::id(),
        liquidity_oracle_program_id: everlend_liquidity_oracle::id(),
        depositor_program_id: everlend_depositor::id(),
        income_pools_program_id: everlend_income_pools::id(),
        money_market_program_ids: [Pubkey::default(); TOTAL_DISTRIBUTIONS],
    };
    programs.money_market_program_ids[0] = default_accounts.port_finance.program_id;
    programs.money_market_program_ids[1] = default_accounts.larix.program_id;
    programs.money_market_program_ids[2] = default_accounts.solend.program_id;

    println!("programs = {:#?}", programs);

    let mut collateral_pool_markets: [Pubkey; TOTAL_DISTRIBUTIONS] = Default::default();
    collateral_pool_markets[..mm_collateral_pool_markets.len()]
        .copy_from_slice(&mm_collateral_pool_markets);

    let roots = RegistryRootAccounts {
        general_pool_market: general_pool_market_pubkey,
        income_pool_market: income_pool_market_pubkey,
        collateral_pool_markets,
        liquidity_oracle: liquidity_oracle_pubkey,
    };

    println!("roots = {:#?}", &roots);

    registry::set_registry_config(
        config,
        &registry_pubkey,
        programs,
        roots,
        RegistrySettings {
            refresh_income_interval: 0,
        },
    )?;

    println!("Depositor");
    let depositor_pubkey = depositor::init(config, &registry_pubkey, None, rebalance_executor)?;

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
            .zip(mm_collateral_pool_markets.iter())
            .filter_map(|(collateral_mint, mm_pool_market_pubkey)| {
                collateral_mint.map(|coll_mint| (coll_mint, *mm_pool_market_pubkey))
            })
            .collect();

        let (general_pool_pubkey, general_pool_token_account, general_pool_mint) =
            general_pool::create_pool(config, &general_pool_market_pubkey, mint)?;

        registry::set_registry_pool_config(
            config,
            &registry_pubkey,
            &general_pool_pubkey,
            SetRegistryPoolConfigParams {
                deposit_minimum: 0,
                withdraw_minimum: 0,
            },
        )?;

        let token_account = get_associated_token_address(&payer_pubkey, mint);
        let pool_account =
            spl_create_associated_token_account(config, &payer_pubkey, &general_pool_mint)?;

        let (income_pool_pubkey, income_pool_token_account) =
            income_pools::create_pool(config, &income_pool_market_pubkey, mint)?;

        // MM Pools
        let mm_pool_collection = collateral_mints
            .iter()
            .map(
                |(collateral_mint, mm_pool_market_pubkey)| -> Result<PoolPubkeys, ClientError> {
                    println!("MM Pool: {}", collateral_mint);
                    let pool_pubkeys = collateral_pool::create_pool(
                        config,
                        mm_pool_market_pubkey,
                        collateral_mint,
                    )?;
                    collateral_pool::create_pool_withdraw_authority(
                        config,
                        mm_pool_market_pubkey,
                        &pool_pubkeys.pool,
                        depositor_authority,
                        &config.fee_payer.pubkey(),
                    )?;

                    Ok(pool_pubkeys)
                },
            )
            .collect::<Result<Vec<PoolPubkeys>, ClientError>>()?;

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

        let collateral_pools = collateral_mints
            .iter()
            .zip(mm_pool_collection)
            .map(
                |((collateral_mint, _mm_pool_market_pubkey), mm_pool_pubkeys)| {
                    CollateralPoolAccounts {
                        pool: mm_pool_pubkeys.pool,
                        pool_token_account: mm_pool_pubkeys.token_account,
                        token_mint: *collateral_mint,
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
                mm_pools: Vec::new(),
                mining_accounts: Vec::new(),
                collateral_pools,
                liquidity_transit: liquidity_transit_pubkey,
                port_finance_obligation_account: Pubkey::default(),
            },
        );
    }

    let initialized_accounts = InitializedAccounts {
        payer: payer_pubkey,
        registry: registry_pubkey,
        general_pool_market: general_pool_market_pubkey,
        income_pool_market: income_pool_market_pubkey,
        mm_pool_markets: Vec::new(),
        collateral_pool_markets: mm_collateral_pool_markets,
        token_accounts,
        liquidity_oracle: liquidity_oracle_pubkey,
        depositor: depositor_pubkey,
        quarry_mining: BTreeMap::new(),
        rebalance_executor,
    };

    initialized_accounts.save(accounts_path).unwrap();

    Ok(())
}

async fn command_create_collateral_pools(
    config: &Config,
    accounts_path: &str,
) -> anyhow::Result<()> {
    let collateral_pool_markets = vec![
        collateral_pool::create_market(config, None)?,
        collateral_pool::create_market(config, None)?,
        collateral_pool::create_market(config, None)?,
    ];
    let mut initialized_accounts = InitializedAccounts::load(accounts_path).unwrap();
    initialized_accounts.collateral_pool_markets = collateral_pool_markets;

    let default_accounts = config.get_default_accounts();

    let (_, collateral_mint_map) = get_asset_maps(default_accounts.clone());

    let mut collateral_pool_markets: [Pubkey; TOTAL_DISTRIBUTIONS] = Default::default();
    collateral_pool_markets[..initialized_accounts.collateral_pool_markets.len()]
        .copy_from_slice(&initialized_accounts.collateral_pool_markets);

    let token_accounts = initialized_accounts.token_accounts.iter_mut();
    let depositor_pubkey = &initialized_accounts.depositor;
    for pair in token_accounts {
        let collateral_mints: Vec<(Pubkey, Pubkey)> = collateral_mint_map
            .get(pair.0)
            .unwrap()
            .iter()
            .zip(initialized_accounts.collateral_pool_markets.iter())
            .filter_map(|(collateral_mint, mm_pool_market_pubkey)| {
                collateral_mint.map(|coll_mint| (coll_mint, *mm_pool_market_pubkey))
            })
            .collect();

        let mm_pool_collection = collateral_mints
            .iter()
            .map(|(collateral_mint, mm_pool_market_pubkey)| {
                if !collateral_mint
                    .eq(&Pubkey::from_str("11111111111111111111111111111111").unwrap())
                {
                    println!("MM Pool: {}", collateral_mint);
                    collateral_pool::create_pool(config, mm_pool_market_pubkey, collateral_mint)
                } else {
                    Ok(PoolPubkeys {
                        pool: Pubkey::from_str("11111111111111111111111111111111").unwrap(),
                        token_account: Pubkey::from_str("11111111111111111111111111111111")
                            .unwrap(),
                    })
                }
            })
            .collect::<Result<Vec<PoolPubkeys>, ClientError>>()?;
        collateral_mints
            .iter()
            .map(|(collateral_mint, _mm_pool_market_pubkey)| {
                if !collateral_mint
                    .eq(&Pubkey::from_str("11111111111111111111111111111111").unwrap())
                {
                    depositor::create_transit(config, depositor_pubkey, collateral_mint, None)
                } else {
                    Ok(Pubkey::from_str("11111111111111111111111111111111").unwrap())
                }
            })
            .collect::<Result<Vec<Pubkey>, ClientError>>()?;

        let collateral_pools = collateral_mints
            .iter()
            .zip(mm_pool_collection)
            .map(
                |((collateral_mint, _mm_pool_market_pubkey), mm_pool_pubkeys)| {
                    CollateralPoolAccounts {
                        pool: mm_pool_pubkeys.pool,
                        pool_token_account: mm_pool_pubkeys.token_account,
                        token_mint: *collateral_mint,
                    }
                },
            )
            .collect();

        let mut accounts = pair.1;
        accounts.collateral_pools = collateral_pools;
    }
    initialized_accounts.save(accounts_path).unwrap();
    Ok(())
}

async fn create_pool_withdraw_authority(
    config: &Config,
    accounts_path: &str,
) -> anyhow::Result<()> {
    let mut initialized_accounts = InitializedAccounts::load(accounts_path).unwrap_or_default();
    let pool_markets = initialized_accounts.collateral_pool_markets;
    let depositor = initialized_accounts.depositor;
    let token_accounts = initialized_accounts.token_accounts.iter_mut();
    for pair in token_accounts {
        pair.1
            .collateral_pools
            .iter()
            .zip(pool_markets.clone())
            .filter(|(keyset, _)| {
                !keyset
                    .pool
                    .eq(&Pubkey::from_str("11111111111111111111111111111111").unwrap())
            })
            .map(|(keyset, market)| {
                let (depositor_authority, _) =
                    find_program_address(&everlend_depositor::id(), &depositor);
                collateral_pool::create_pool_withdraw_authority(
                    config,
                    &market,
                    &keyset.pool,
                    &depositor_authority,
                    &config.fee_payer.pubkey(),
                )
            })
            .collect::<Result<Vec<Pubkey>, ClientError>>()?;
    }
    Ok(())
}

async fn command_info(config: &Config, accounts_path: &str) -> anyhow::Result<()> {
    let initialized_accounts = InitializedAccounts::load(accounts_path).unwrap_or_default();
    let default_accounts = config.get_default_accounts();

    println!("fee_payer: {:?}", config.fee_payer.pubkey());
    println!("default_accounts = {:#?}", default_accounts);
    println!("{:#?}", initialized_accounts);

    println!(
        "{:#?}",
        get_general_pool_market(config, &initialized_accounts.general_pool_market)?
    );

    for (_, token_accounts) in initialized_accounts.token_accounts {
        println!("mint = {:?}", token_accounts.mint);
        let (withdraw_requests_pubkey, withdraw_requests) = get_withdrawal_requests(
            config,
            &initialized_accounts.general_pool_market,
            &token_accounts.mint,
        )?;
        println!("{:#?}", (withdraw_requests_pubkey, &withdraw_requests));

        let (rebalancing_pubkey, _) = find_rebalancing_program_address(
            &everlend_depositor::id(),
            &initialized_accounts.depositor,
            &token_accounts.mint,
        );

        let rebalancing = config.get_account_unpack::<Rebalancing>(&rebalancing_pubkey)?;
        println!("{:#?}", (rebalancing_pubkey, rebalancing));
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

    let _token = initialiazed_accounts
        .token_accounts
        .get(&case.unwrap())
        .unwrap();

    println!("Migrate withdraw requests");
    general_pool::migrate_general_pool_account(config)?;
    println!("Finished!");

    Ok(())
}

async fn command_run_migrate_pool_market(
    config: &Config,
    accounts_path: &str,
    keypair: Keypair,
) -> anyhow::Result<()> {
    let initialized_accounts = InitializedAccounts::load(accounts_path).unwrap();

    println!("Close general pool market");
    println!(
        "pool market id: {}",
        &initialized_accounts.general_pool_market
    );
    general_pool::close_pool_market_account(config, &initialized_accounts.general_pool_market)?;
    println!("Closed general pool market");

    println!("Create general pool market");
    general_pool::create_market(config, Some(keypair), &initialized_accounts.registry)?;
    println!("Finished!");

    Ok(())
}

async fn command_create_depositor_transit_account(
    config: &Config,
    token_mint: Pubkey,
    seed: Option<String>,
) -> anyhow::Result<()> {
    let initialized_accounts = config.get_initialized_accounts();

    println!("Token mint {}. Seed {:?}", token_mint, seed);
    depositor::create_transit(config, &initialized_accounts.depositor, &token_mint, seed)?;

    Ok(())
}

// TODO remove after setup
async fn command_create_income_pool_safety_fund_token_account(
    config: &Config,
    accounts_path: &str,
    case: Option<String>,
) -> anyhow::Result<()> {
    let initialiazed_accounts = InitializedAccounts::load(accounts_path).unwrap_or_default();

    if case.is_none() {
        println!("Token mint not presented");
        return Ok(());
    }

    let token = initialiazed_accounts
        .token_accounts
        .get(&case.unwrap())
        .unwrap();

    println!("Create income pool safety fund token account");
    income_pools::create_income_pool_safety_fund_token_account(
        config,
        &initialiazed_accounts.income_pool_market,
        &token.mint,
    )?;
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
            SubCommand::with_name("set-registry-config")
                .about("Set a new registry config")
                .arg(
                    Arg::with_name("registry")
                        .long("registry")
                        .validator(is_pubkey)
                        .value_name("ADDRESS")
                        .takes_value(true)
                        .required(true)
                        .help("Registry pubkey"),
                ),
        )
        .subcommand(
            SubCommand::with_name("init-mining")
                .arg(
                    Arg::with_name("staking-money-market")
                        .long("staking-money-market")
                        .value_name("NUMBER")
                        .takes_value(true)
                        .required(true)
                        .help("Money market index"),
                )
                .arg(
                    Arg::with_name("token")
                        .long("token")
                        .short("t")
                        .value_name("TOKEN")
                        .takes_value(true)
                        .required(true)
                        .help("Token"),
                )
                .arg(
                    Arg::with_name("sub-reward-mint")
                        .long("sub-reward-mint")
                        .short("m")
                        .value_name("REWARD_MINT")
                        .takes_value(true)
                        .help("Sub reward token mint"),
                ),
        )
        .subcommand(SubCommand::with_name("save-larix-accounts"))
        .subcommand(SubCommand::with_name("test-larix-mining-raw"))
        .subcommand(SubCommand::with_name("save-quarry-accounts"))
        .subcommand(
            SubCommand::with_name("init-quarry-mining-accounts")
                .arg(
                    Arg::with_name("default")
                        .long("default")
                        .value_name("PATH")
                        .takes_value(true)
                        .help("Defaults file"),
                )
                .arg(
                    Arg::with_name("token")
                        .long("token")
                        .short("t")
                        .value_name("TOKEN")
                        .takes_value(true)
                        .required(true)
                        .help("Token"),
                ),
        )
        .subcommand(
            SubCommand::with_name("test-quarry-mining-raw").arg(
                Arg::with_name("token")
                    .long("token")
                    .short("t")
                    .value_name("TOKEN")
                    .takes_value(true)
                    .required(true)
                    .help("Token"),
            ),
        )
        .subcommand(
            SubCommand::with_name("set-registry-pool-config")
                .about("Set a new registry pool config")
                .arg(
                    Arg::with_name("accounts")
                        .short("A")
                        .long("accounts")
                        .value_name("PATH")
                        .takes_value(true)
                        .help("Accounts file"),
                )
                .arg(
                    Arg::with_name("general-pool")
                        .long("general-pool")
                        .short("P")
                        .validator(is_pubkey)
                        .value_name("ADDRESS")
                        .takes_value(true)
                        .required(true)
                        .help("General pool pubkey"),
                )
                .arg(
                    Arg::with_name("min-deposit")
                        .long("min-deposit")
                        .value_name("NUMBER")
                        .takes_value(true)
                        .help("Minimum amount for deposit"),
                )
                .arg(
                    Arg::with_name("min-withdraw")
                        .long("min-withdraw")
                        .value_name("NUMBER")
                        .takes_value(true)
                        .help("Minimum amount for deposit"),
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
            SubCommand::with_name("update-liquidity-oracle-authority")
                .about("Update liquidity oracle authority")
                .arg(
                    Arg::with_name("authority")
                        .long("authority")
                        .validator(is_keypair)
                        .value_name("AUTHORITY")
                        .takes_value(true)
                        .required(true)
                        .help("Old manager keypair"),
                )
                .arg(
                    Arg::with_name("new-authority")
                        .long("new-authority")
                        .validator(is_keypair)
                        .value_name("NEW-AUTHORITY")
                        .takes_value(true)
                        .required(true)
                        .help("New manager keypair"),
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
                )
                .arg(
                    Arg::with_name("rebalance-executor")
                        .long("rebalance-executor")
                        .validator(is_pubkey)
                        .value_name("PUBKEY")
                        .takes_value(true)
                        .help("Rebalance executor pubkey"),
                ),
        )
        .subcommand(
            SubCommand::with_name("create-mm-pool")
                .about("Create a new MM pool")
                .arg(
                    Arg::with_name("money-market")
                        .long("money-market")
                        .value_name("NUMBER")
                        .takes_value(true)
                        .required(true)
                        .help("Money market index"),
                )
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
            SubCommand::with_name("create-collateral-pools")
                .about("Create collateral pools")
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
            SubCommand::with_name("create-pool-withdraw-authority")
                .about("Create pool withdraw authority")
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
            SubCommand::with_name("cancel-withdraw-request")
                .about("Cancel withdraw request")
                .arg(
                    Arg::with_name("request")
                        .long("request")
                        .validator(is_pubkey)
                        .value_name("ADDRESS")
                        .takes_value(true)
                        .required(true)
                        .help("Withdrawal request pubkey"),
                ),
        )
        .subcommand(
            SubCommand::with_name("reset-rebalancing")
                .about("Reset rebalancing")
                .arg(
                    Arg::with_name("rebalancing")
                        .long("rebalancing")
                        .validator(is_pubkey)
                        .value_name("ADDRESS")
                        .takes_value(true)
                        .required(true)
                        .help("Rebalancing pubkey"),
                )
                .arg(
                    Arg::with_name("amount")
                        .long("amount")
                        .validator(is_amount)
                        .value_name("NUMBER")
                        .takes_value(true)
                        .required(true)
                        .help("Liquidity amount"),
                )
                .arg(
                    Arg::with_name("distribution")
                        .long("distribution")
                        .multiple(true)
                        .value_name("DISTRIBUTION")
                        .short("d")
                        .number_of_values(10)
                        .required(true)
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
        .subcommand(SubCommand::with_name("info-reserve-liquidity").about("Info reserve accounts"))
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
                )
                .arg(
                    Arg::with_name("rebalance-executor")
                        .long("rebalance-executor")
                        .validator(is_pubkey)
                        .value_name("PUBKEY")
                        .takes_value(true)
                        .help("Rebalance executor pubkey"),
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
            SubCommand::with_name("migration")
                .about("Migrations")
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
                .subcommand(
                    SubCommand::with_name("migrate-pool-market")
                        .about("Migrate pool market account")
                        .arg(
                            Arg::with_name("accounts")
                                .short("A")
                                .long("accounts")
                                .value_name("PATH")
                                .takes_value(true)
                                .help("Accounts file"),
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
                .subcommand(SubCommand::with_name("migrate-depositor").about(
                    "Migrate Depositor account. Must be invoke after migrate-registry-config.",
                ))
                .subcommand(
                    SubCommand::with_name("migrate-registry-config").about(
                        "Migrate RegistryConfig account. Must be invoke by registry manager.",
                    ),
                ),
        )
        .subcommand(
            SubCommand::with_name("create-safety-fund-token-account")
                .about("Run  create income pool safety fund token account")
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
            SubCommand::with_name("create-depositor-transit-token-account")
                .about("Run  create depositor transit token account")
                .arg(
                    Arg::with_name("seed")
                        .long("seed")
                        .value_name("SEED")
                        .takes_value(true)
                        .help("Transit seed"),
                )
                .arg(
                    Arg::with_name("token-mint")
                        .long("token-mint")
                        .value_name("MINT")
                        .validator(is_pubkey)
                        .takes_value(true)
                        .help("Rewards token mint"),
                ),
        )
        .get_matches();

    let mut wallet_manager = None;
    let config = {
        let cli_config = if let Some(config_file) = matches.value_of("config_file") {
            println!("config_file = {:?}", config_file);
            solana_cli_config::Config::load(config_file)
                .map(|mut cfg| {
                    let path = PathBuf::from(cfg.keypair_path.clone());
                    if !path.is_absolute() {
                        let mut keypair_path = dirs_next::home_dir().expect("home directory");
                        keypair_path.push(path);
                        cfg.keypair_path = keypair_path.to_str().unwrap().to_string();
                    }

                    cfg
                })
                .unwrap_or_default()
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

        println!("fee_payer = {:?}", fee_payer);
        println!("owner = {:?}", owner);

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
        ("set-registry-config", Some(arg_matches)) => {
            let registry_pubkey = pubkey_of(arg_matches, "registry").unwrap();
            command_set_registry_config(&config, registry_pubkey).await
        }
        ("init-mining", Some(arg_matches)) => {
            let staking_money_market =
                value_of::<usize>(arg_matches, "staking-money-market").unwrap();
            let token = value_of::<String>(arg_matches, "token").unwrap();
            let sub_reward_mint = pubkey_of(arg_matches, "sub-reward-mint");
            command_init_mining(
                &config,
                StakingMoneyMarket::from(staking_money_market),
                &token,
                sub_reward_mint,
            )
        }
        ("save-larix-accounts", Some(_)) => {
            command_save_larix_accounts("../tests/tests/fixtures/larix/reserve_sol.bin").await
        }
        ("test-larix-mining-raw", Some(_)) => command_test_larix_mining_raw(&config),
        ("save-quarry-accounts", Some(_)) => command_save_quarry_accounts(&config).await,
        ("init-quarry-mining-accounts", Some(arg_matches)) => {
            let token = value_of::<String>(arg_matches, "token").unwrap();
            command_init_quarry_mining_accounts(&config, &token)
        }
        ("test-quarry-mining-raw", Some(arg_matches)) => {
            let token = value_of::<String>(arg_matches, "token").unwrap();
            command_test_quarry_mining_raw(&config, &token)
        }
        ("set-registry-pool-config", Some(arg_matches)) => {
            let accounts_path = arg_matches.value_of("accounts").unwrap_or("accounts.yaml");
            let general_pool = pubkey_of(arg_matches, "general-pool").unwrap();
            let deposit_minimum = value_of::<u64>(arg_matches, "min-deposit").unwrap_or(0);
            let withdraw_minimum = value_of::<u64>(arg_matches, "min-withdraw").unwrap_or(0);
            let params = SetRegistryPoolConfigParams {
                deposit_minimum,
                withdraw_minimum,
            };
            command_set_registry_pool_config(&config, accounts_path, general_pool, params).await
        }
        ("create-general-pool-market", Some(arg_matches)) => {
            let keypair = keypair_of(arg_matches, "keypair");
            let registry_pubkey = pubkey_of(arg_matches, "registry").unwrap();
            command_create_general_pool_market(&config, keypair, registry_pubkey).await
        }
        ("create-income-pool-market", Some(arg_matches)) => {
            let keypair = keypair_of(arg_matches, "keypair");
            command_create_income_pool_market(&config, keypair).await
        }
        ("create-mm-pool-market", Some(arg_matches)) => {
            let keypair = keypair_of(arg_matches, "keypair");
            let money_market = value_of::<usize>(arg_matches, "money-market").unwrap();
            command_create_collateral_pool_market(&config, keypair, MoneyMarket::from(money_market))
                .await
        }
        ("create-liquidity-oracle", Some(arg_matches)) => {
            let keypair = keypair_of(arg_matches, "keypair");
            command_create_liquidity_oracle(&config, keypair).await
        }
        ("update-liquidity-oracle-authority", Some(arg_matches)) => {
            let authority = keypair_of(arg_matches, "authority").unwrap();
            let new_authority = keypair_of(arg_matches, "new-authority").unwrap();

            command_update_liquidity_oracle(&config, authority, new_authority).await
        }
        ("create-depositor", Some(arg_matches)) => {
            let keypair = keypair_of(arg_matches, "keypair");
            let executor_pubkey = pubkey_of(arg_matches, "rebalance-executor").unwrap();
            command_create_depositor(&config, keypair, executor_pubkey).await
        }
        ("create-mm-pool", Some(arg_matches)) => {
            let money_market = value_of::<usize>(arg_matches, "money-market").unwrap();
            let mints: Vec<_> = arg_matches.values_of("mints").unwrap().collect();
            command_create_collateral_pool(&config, MoneyMarket::from(money_market), mints).await
        }
        ("create-collateral-pools", Some(arg_matches)) => {
            let accounts_path = arg_matches.value_of("accounts").unwrap_or("accounts.yaml");
            command_create_collateral_pools(&config, accounts_path).await
        }
        ("create-pool-withdraw-authority", Some(arg_matches)) => {
            let accounts_path = arg_matches.value_of("accounts").unwrap_or("accounts.yaml");
            create_pool_withdraw_authority(&config, accounts_path).await
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
        ("cancel-withdraw-request", Some(arg_matches)) => {
            let request_pubkey = pubkey_of(arg_matches, "request").unwrap();
            command_cancel_withdraw_request(&config, &request_pubkey).await
        }
        ("reset-rebalancing", Some(arg_matches)) => {
            let rebalancing_pubkey = pubkey_of(arg_matches, "rebalancing").unwrap();
            let distributed_liquidity = value_of::<u64>(arg_matches, "amount").unwrap();
            let distribution: Vec<u64> = values_of::<u64>(arg_matches, "distribution").unwrap();
            command_reset_rebalancing(
                &config,
                &rebalancing_pubkey,
                distributed_liquidity,
                distribution,
            )
            .await
        }
        ("info-reserve-liquidity", Some(_)) => command_info_reserve_liquidity(&config).await,
        ("create", Some(arg_matches)) => {
            let accounts_path = arg_matches.value_of("accounts").unwrap_or("accounts.yaml");
            let mints: Vec<_> = arg_matches.values_of("mints").unwrap().collect();
            let rebalance_executor_pubkey = pubkey_of(arg_matches, "rebalance-executor").unwrap();
            command_create(&config, accounts_path, mints, rebalance_executor_pubkey).await
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
        ("migration", Some(arg_matches)) => {
            let _ = match arg_matches.subcommand() {
                ("migrate-general-pool", Some(arg_matches)) => {
                    let accounts_path = arg_matches.value_of("accounts").unwrap_or("accounts.yaml");
                    let case = value_of::<String>(arg_matches, "case");
                    command_run_migrate(&config, accounts_path, case).await
                }
                ("migrate-depositor", Some(_)) => {
                    println!("WARN! This migration must be invoke after migrate-registry-config.");
                    println!("Started Depositor migration");
                    command_migrate_depositor(&config).await
                }
                ("migrate-registry-config", Some(_)) => {
                    println!("Started RegistryConfig migration");
                    command_migrate_registry_config(&config).await
                }
                ("migrate-pool-market", Some(arg_matches)) => {
                    let accounts_path = arg_matches.value_of("accounts").unwrap_or("accounts.yaml");
                    let keypair = keypair_of(arg_matches, "keypair").unwrap();
                    command_run_migrate_pool_market(&config, accounts_path, keypair).await
                }
                _ => unreachable!(),
            }
            .map_err(|err| {
                eprintln!("{}", err);
                exit(1);
            });
            Ok(())
        }
        // TODO remove after migration
        ("create-safety-fund-token-account", Some(arg_matches)) => {
            let accounts_path = arg_matches.value_of("accounts").unwrap_or("accounts.yaml");
            let case = value_of::<String>(arg_matches, "case");
            command_create_income_pool_safety_fund_token_account(&config, accounts_path, case).await
        }
        ("create-depositor-transit-token-account", Some(arg_matches)) => {
            let token_mint = pubkey_of(arg_matches, "token-mint").unwrap();
            let seed = value_of::<String>(arg_matches, "seed");
            command_create_depositor_transit_account(&config, token_mint, seed).await
        }
        _ => unreachable!(),
    }
    .map_err(|err| {
        eprintln!("{}", err);
        exit(1);
    });

    Ok(())
}
