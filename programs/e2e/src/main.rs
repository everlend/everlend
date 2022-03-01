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
use everlend_depositor::state::Rebalancing;
use everlend_liquidity_oracle::state::DistributionArray;
use everlend_registry::state::{SetRegistryConfigParams, TOTAL_DISTRIBUTIONS};
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

async fn command_create(config: &Config) -> anyhow::Result<()> {
    let payer_pubkey = config.fee_payer.pubkey();
    println!("Fee payer: {}", payer_pubkey);

    let sol_mint = Pubkey::from_str(SOL_MINT).unwrap();

    let port_finance_sol_collateral_mint =
        Pubkey::from_str(PORT_FINANCE_RESERVE_SOL_COLLATERAL_MINT).unwrap();
    let larix_sol_collateral_mint = Pubkey::from_str(LARIX_RESERVE_SOL_COLLATERAL_MINT).unwrap();
    let port_finance_program_id = Pubkey::from_str(integrations::PORT_FINANCE_PROGRAM_ID).unwrap();
    let larix_program_id = Pubkey::from_str(integrations::LARIX_PROGRAM_ID).unwrap();

    println!("0. Registry");
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

    println!("1. General pool");
    let pool_market_pubkey = general_pool::create_market(config, None)?;
    let (pool_pubkey, pool_token_account, pool_mint) =
        general_pool::create_pool(config, &pool_market_pubkey, &sol_mint)?;

    let token_account = get_associated_token_address(&payer_pubkey, &sol_mint);
    let pool_account = spl_create_associated_token_account(config, &payer_pubkey, &pool_mint)?;

    println!("1.1. Income pool");
    let income_pool_market_pubkey = income_pools::create_market(config, None, &pool_market_pubkey)?;
    let (income_pool_pubkey, income_pool_token_account) =
        income_pools::create_pool(config, &income_pool_market_pubkey, &sol_mint)?;

    println!("2. Deposit liquidity");
    general_pool::deposit(
        config,
        &pool_market_pubkey,
        &pool_pubkey,
        &token_account,
        &pool_account,
        &pool_token_account,
        &pool_mint,
        1000,
    )?;

    println!("3. MM Pool: Port Finance");
    let port_finance_mm_pool_market_pubkey = ulp::create_market(config, None)?;
    let (
        port_finance_mm_pool_pubkey,
        port_finance_mm_pool_token_account,
        port_finance_mm_pool_mint,
    ) = ulp::create_pool(
        config,
        &port_finance_mm_pool_market_pubkey,
        &port_finance_sol_collateral_mint,
    )?;

    println!("3.1 MM Pool: Larix");
    let larix_mm_pool_market_pubkey = ulp::create_market(config, None)?;
    let (larix_mm_pool_pubkey, larix_mm_pool_token_account, larix_mm_pool_mint) = ulp::create_pool(
        config,
        &larix_mm_pool_market_pubkey,
        &larix_sol_collateral_mint,
    )?;

    println!("4. Liquidity oracle");
    let liquidity_oracle_pubkey = liquidity_oracle::init(config, None)?;
    let mut distribution = DistributionArray::default();
    distribution[0] = 0;
    distribution[1] = 0;

    liquidity_oracle::create_token_distribution(
        config,
        &liquidity_oracle_pubkey,
        &sol_mint,
        &distribution,
    )?;

    println!("5. Depositor");
    let depositor_pubkey = depositor::init(
        config,
        &registry_pubkey,
        None,
        &pool_market_pubkey,
        &income_pool_market_pubkey,
        &liquidity_oracle_pubkey,
    )?;

    let liquidity_transit_pubkey = depositor::create_transit(config, &depositor_pubkey, &sol_mint)?;
    depositor::create_transit(config, &depositor_pubkey, &port_finance_sol_collateral_mint)?;
    depositor::create_transit(config, &depositor_pubkey, &larix_sol_collateral_mint)?;
    depositor::create_transit(config, &depositor_pubkey, &port_finance_mm_pool_mint)?;
    depositor::create_transit(config, &depositor_pubkey, &larix_mm_pool_mint)?;

    println!("6. Prepare borrow authority");
    let (depositor_authority, _) =
        &everlend_utils::find_program_address(&everlend_depositor::id(), &depositor_pubkey);
    general_pool::create_pool_borrow_authority(
        config,
        &pool_market_pubkey,
        &pool_pubkey,
        depositor_authority,
        10_000, // 100%
    )?;

    let mut token_accounts = HashMap::new();
    token_accounts.insert(
        "SOL".to_string(),
        TokenAccounts {
            mint: sol_mint,
            liquidity_token_account: token_account,
            collateral_token_account: pool_account,
            general_pool: pool_pubkey,
            general_pool_token_account: pool_token_account,
            general_pool_mint: pool_mint,
            income_pool: income_pool_pubkey,
            income_pool_token_account,
            mm_pools: vec![
                MoneyMarketAccounts {
                    pool: port_finance_mm_pool_pubkey,
                    pool_token_account: port_finance_mm_pool_token_account,
                    token_mint: port_finance_sol_collateral_mint,
                    pool_mint: port_finance_mm_pool_mint,
                },
                MoneyMarketAccounts {
                    pool: larix_mm_pool_pubkey,
                    pool_token_account: larix_mm_pool_token_account,
                    token_mint: larix_sol_collateral_mint,
                    pool_mint: larix_mm_pool_mint,
                },
            ],
            liquidity_transit: liquidity_transit_pubkey,
        },
    );
    let initialiazed_accounts = InitializedAccounts {
        payer: payer_pubkey,
        registry: registry_pubkey,
        general_pool_market: pool_market_pubkey,
        income_pool_market: income_pool_market_pubkey,
        mm_pool_markets: vec![
            port_finance_mm_pool_market_pubkey,
            larix_mm_pool_market_pubkey,
        ],
        token_accounts,
        liquidity_oracle: liquidity_oracle_pubkey,
        depositor: depositor_pubkey,
    };

    initialiazed_accounts.save("accounts_config.yaml").unwrap();

    Ok(())
}

async fn command_info(config: &Config) -> anyhow::Result<()> {
    let initialiazed_accounts =
        InitializedAccounts::load("accounts_config.yaml").unwrap_or_default();

    println!("{:#?}", initialiazed_accounts);

    Ok(())
}

async fn command_run_test(config: &Config, case: Option<String>) -> anyhow::Result<()> {
    println!("Run {:?}", case);

    let initialiazed_accounts =
        InitializedAccounts::load("accounts_config.yaml").unwrap_or_default();

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

    let sol = token_accounts.get("SOL").unwrap();

    let port_finance_program_id = Pubkey::from_str(integrations::PORT_FINANCE_PROGRAM_ID).unwrap();
    let larix_program_id = Pubkey::from_str(integrations::LARIX_PROGRAM_ID).unwrap();

    let sol_oracle = Pubkey::from_str(SOL_ORACLE).unwrap();
    // TODO use for larix pyth or larix oracle?
    let sol_larix_oracle = Pubkey::from_str(SOL_LARIX_ORACLE).unwrap();
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

    let mut distribution = DistributionArray::default();

    match case.as_deref() {
        Some("first") => {
            distribution[0] = 1000000000;
            distribution[1] = 0;
            liquidity_oracle::update_token_distribution(
                config,
                &liquidity_oracle,
                &sol.mint,
                &distribution,
            )?;
            println!("Rebalancing: Start");
            let (_, rebalancing) = depositor::start_rebalancing(
                config,
                &registry,
                &depositor,
                &sol.mint,
                &general_pool_market,
                &sol.general_pool_token_account,
                &liquidity_oracle,
            )?;

            println!("Rebalancing: Deposit: Port Finance");
            depositor::deposit(
                config,
                &registry,
                &depositor,
                &mm_pool_markets[0],
                &sol.mm_pools[0].pool_token_account,
                &sol.mint,
                &sol.mm_pools[0].token_mint,
                &sol.mm_pools[0].pool_mint,
                &port_finance_program_id,
                integrations::deposit_accounts(
                    &port_finance_program_id,
                    &MoneyMarketPubkeys::SPL(port_finance_pubkeys.clone()),
                ),
            )?;

            let mut balance = config
                .rpc_client
                .get_token_account_balance(&sol.liquidity_transit)?
                .amount;
            println!("balance 0 = {:?}", balance);

            distribution[0] = 999876767;
            distribution[1] = 0;
            liquidity_oracle::update_token_distribution(
                config,
                &liquidity_oracle,
                &sol.mint,
                &distribution,
            )?;
            println!("Rebalancing: Start");
            let (_, rebalancing) = depositor::start_rebalancing(
                config,
                &registry,
                &depositor,
                &sol.mint,
                &general_pool_market,
                &sol.general_pool_token_account,
                &liquidity_oracle,
            )?;

            println!("{:#?}", rebalancing);

            balance = config
                .rpc_client
                .get_token_account_balance(&sol.liquidity_transit)?
                .amount;
            println!("balance 1 = {:?}", balance);

            println!("Rebalancing: Withdraw: Port Finance");
            depositor::withdraw(
                config,
                &registry,
                &depositor,
                &income_pool_market,
                &sol.income_pool_token_account,
                &mm_pool_markets[0],
                &sol.mm_pools[0].pool_token_account,
                &sol.mm_pools[0].token_mint,
                &sol.mint,
                &sol.mm_pools[0].pool_mint,
                &port_finance_program_id,
                integrations::withdraw_accounts(
                    &port_finance_program_id,
                    &MoneyMarketPubkeys::SPL(port_finance_pubkeys.clone()),
                ),
            )?;

            balance = config
                .rpc_client
                .get_token_account_balance(&sol.liquidity_transit)?
                .amount;

            println!("balance 2 = {:?}", balance);

            liquidity_oracle::update_token_distribution(
                config,
                &liquidity_oracle,
                &sol.mint,
                &distribution,
            )?;
            println!("Rebalancing: Start");
            let (_, rebalancing) = depositor::start_rebalancing(
                config,
                &registry,
                &depositor,
                &sol.mint,
                &general_pool_market,
                &sol.general_pool_token_account,
                &liquidity_oracle,
            )?;

            println!("{:#?}", rebalancing);

            balance = config
                .rpc_client
                .get_token_account_balance(&sol.liquidity_transit)?
                .amount;

            println!("balance 3 = {:?}", balance);
        }
        None => {
            distribution[0] = 500_000_000u64;
            distribution[1] = 500_000_000u64;
            liquidity_oracle::update_token_distribution(
                config,
                &liquidity_oracle,
                &sol.mint,
                &distribution,
            )?;
            println!("Rebalancing: Start");
            let (_, rebalancing) = depositor::start_rebalancing(
                config,
                &registry,
                &depositor,
                &sol.mint,
                &general_pool_market,
                &sol.general_pool_token_account,
                &liquidity_oracle,
            )?;

            println!("7.1 Rebalancing: Deposit: Port Finance");
            depositor::deposit(
                config,
                &registry,
                &depositor,
                &mm_pool_markets[0],
                &sol.mm_pools[0].pool_token_account,
                &sol.mint,
                &sol.mm_pools[0].token_mint,
                &sol.mm_pools[0].pool_mint,
                &port_finance_program_id,
                integrations::deposit_accounts(
                    &port_finance_program_id,
                    &MoneyMarketPubkeys::SPL(port_finance_pubkeys.clone()),
                ),
            )?;

            println!("7.2 Rebalancing: Deposit: Larix");
            depositor::deposit(
                config,
                &registry,
                &depositor,
                &mm_pool_markets[1],
                &sol.mm_pools[1].pool_token_account,
                &sol.mint,
                &sol.mm_pools[1].token_mint,
                &sol.mm_pools[1].pool_mint,
                &larix_program_id,
                integrations::deposit_accounts(
                    &larix_program_id,
                    &MoneyMarketPubkeys::Larix(larix_pubkeys.clone()),
                ),
            )?;

            let mut balance = config
                .rpc_client
                .get_token_account_balance(&sol.liquidity_transit)?;
            println!("balance 0 = {:?}", balance);

            // Withdraw request
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
                100,
                1,
            )?;

            println!("8. Update token distribution");
            distribution[0] = 300_000_000u64; // 30%
            distribution[1] = 600_000_000u64; // 60%
            liquidity_oracle::update_token_distribution(
                config,
                &liquidity_oracle,
                &sol.mint,
                &distribution,
            )?;
            println!("Rebalancing: Start");
            let (_, rebalancing) = depositor::start_rebalancing(
                config,
                &registry,
                &depositor,
                &sol.mint,
                &general_pool_market,
                &sol.general_pool_token_account,
                &liquidity_oracle,
            )?;

            println!("{:#?}", rebalancing);

            balance = config
                .rpc_client
                .get_token_account_balance(&sol.liquidity_transit)?;
            println!("balance 1 = {:?}", balance);

            println!("8.2. Rebalancing: Withdraw: Port Finance");
            depositor::withdraw(
                config,
                &registry,
                &depositor,
                &income_pool_market,
                &sol.income_pool_token_account,
                &mm_pool_markets[0],
                &sol.mm_pools[0].pool_token_account,
                &sol.mm_pools[0].token_mint,
                &sol.mint,
                &sol.mm_pools[0].pool_mint,
                &port_finance_program_id,
                integrations::withdraw_accounts(
                    &port_finance_program_id,
                    &MoneyMarketPubkeys::SPL(port_finance_pubkeys.clone()),
                ),
            )?;

            balance = config
                .rpc_client
                .get_token_account_balance(&sol.liquidity_transit)?;

            println!("balance 2 = {:?}", balance);

            println!("8.3. Rebalancing: Deposit: Larix");
            depositor::deposit(
                config,
                &registry,
                &depositor,
                &mm_pool_markets[1],
                &sol.mm_pools[1].pool_token_account,
                &sol.mint,
                &sol.mm_pools[1].token_mint,
                &sol.mm_pools[1].pool_mint,
                &larix_program_id,
                integrations::deposit_accounts(
                    &larix_program_id,
                    &MoneyMarketPubkeys::Larix(larix_pubkeys.clone()),
                ),
            )?;

            println!("9. Update token distribution");
            distribution[0] = 000_000_000u64; // 0%
            distribution[1] = 1_000_000_000u64; // 100%
            liquidity_oracle::update_token_distribution(
                config,
                &liquidity_oracle,
                &sol.mint,
                &distribution,
            )?;
            println!("Rebalancing: Start");
            let (_, rebalancing) = depositor::start_rebalancing(
                config,
                &registry,
                &depositor,
                &sol.mint,
                &general_pool_market,
                &sol.general_pool_token_account,
                &liquidity_oracle,
            )?;

            // Withdraw
            println!("Withdraw");
            general_pool::withdraw(
                config,
                &general_pool_market,
                &sol.general_pool,
                &sol.liquidity_token_account,
                &sol.general_pool_token_account,
                &sol.mint,
                &sol.general_pool_mint,
                1,
            )?;

            println!("{:#?}", rebalancing);

            println!("9.2. Rebalancing: Withdraw: Port Finance");
            depositor::withdraw(
                config,
                &registry,
                &depositor,
                &income_pool_market,
                &sol.income_pool_token_account,
                &mm_pool_markets[0],
                &sol.mm_pools[0].pool_token_account,
                &sol.mm_pools[0].token_mint,
                &sol.mint,
                &sol.mm_pools[0].pool_mint,
                &port_finance_program_id,
                integrations::withdraw_accounts(
                    &port_finance_program_id,
                    &MoneyMarketPubkeys::SPL(port_finance_pubkeys.clone()),
                ),
            )?;

            println!("9.3. Rebalancing: Deposit Larix");
            depositor::deposit(
                config,
                &registry,
                &depositor,
                &mm_pool_markets[1],
                &sol.mm_pools[1].pool_token_account,
                &sol.mint,
                &sol.mm_pools[1].token_mint,
                &sol.mm_pools[1].pool_mint,
                &larix_program_id,
                integrations::deposit_accounts(
                    &larix_program_id,
                    &MoneyMarketPubkeys::Larix(larix_pubkeys.clone()),
                ),
            )?;

            println!("10. Update token distribution");
            distribution[0] = 100_000_000u64; // 10%
            distribution[1] = 0u64; // 0%
            liquidity_oracle::update_token_distribution(
                config,
                &liquidity_oracle,
                &sol.mint,
                &distribution,
            )?;
            println!("Rebalancing: Start");
            let (_, rebalancing) = depositor::start_rebalancing(
                config,
                &registry,
                &depositor,
                &sol.mint,
                &general_pool_market,
                &sol.general_pool_token_account,
                &liquidity_oracle,
            )?;

            println!("{:#?}", rebalancing);

            println!("10.2. Rebalancing: Withdraw: Larix");
            depositor::withdraw(
                config,
                &registry,
                &depositor,
                &income_pool_market,
                &sol.income_pool_token_account,
                &mm_pool_markets[1],
                &sol.mm_pools[1].pool_token_account,
                &sol.mm_pools[1].token_mint,
                &sol.mint,
                &sol.mm_pools[1].pool_mint,
                &larix_program_id,
                integrations::withdraw_accounts(
                    &larix_program_id,
                    &MoneyMarketPubkeys::Larix(larix_pubkeys),
                ),
            )?;

            println!("10.3. Rebalancing: Deposit Port Finance");
            depositor::deposit(
                config,
                &registry,
                &depositor,
                &mm_pool_markets[0],
                &sol.mm_pools[0].pool_token_account,
                &sol.mint,
                &sol.mm_pools[0].token_mint,
                &sol.mm_pools[0].pool_mint,
                &port_finance_program_id,
                integrations::deposit_accounts(
                    &port_finance_program_id,
                    &MoneyMarketPubkeys::SPL(port_finance_pubkeys),
                ),
            )?;
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
        .subcommand(SubCommand::with_name("create").about("Create a new accounts"))
        .subcommand(SubCommand::with_name("info").about("Print out env information"))
        .subcommand(
            SubCommand::with_name("test").about("Run a test").arg(
                Arg::with_name("case")
                    .value_name("NAME")
                    .takes_value(true)
                    .index(1)
                    .help("Case"),
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
        ("create", Some(arg_matches)) => command_create(&config).await,
        ("info", Some(arg_matches)) => command_info(&config).await,
        ("test", Some(arg_matches)) => {
            let case = value_of::<String>(arg_matches, "case");
            command_run_test(&config, case).await
        }
        _ => unreachable!(),
    }
    .map_err(|err| {
        eprintln!("{}", err);
        exit(1);
    });

    Ok(())
}
