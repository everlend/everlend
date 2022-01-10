use clap::{
    crate_description, crate_name, crate_version, value_t, App, AppSettings, Arg, SubCommand,
};
use everlend_income_pools::{
    instruction,
    state::{AccountType, IncomePool, IncomePoolMarket},
};
use solana_account_decoder::UiAccountEncoding;
use solana_clap_utils::{
    fee_payer::fee_payer_arg,
    input_parsers::{keypair_of, pubkey_of},
    input_validators::{is_keypair, is_keypair_or_ask_keyword, is_pubkey, is_url_or_moniker},
    keypair::signer_from_path,
};
use solana_client::{
    client_error::ClientError,
    rpc_client::RpcClient,
    rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
    rpc_filter::{Memcmp, MemcmpEncodedBytes, MemcmpEncoding, RpcFilterType},
};
use solana_program::{
    native_token::lamports_to_sol, program_pack::Pack, pubkey::Pubkey, system_instruction,
};
use solana_sdk::{
    account::Account, commitment_config::CommitmentConfig, signature::Keypair, signer::Signer,
    transaction::Transaction,
};
use std::{env, process::exit};

#[allow(dead_code)]
struct Config {
    rpc_client: RpcClient,
    verbose: bool,
    owner: Box<dyn Signer>,
    fee_payer: Box<dyn Signer>,
}

type Error = Box<dyn std::error::Error>;
type CommandResult = Result<Option<Transaction>, Error>;

macro_rules! unique_signers {
    ($vec:ident) => {
        $vec.sort_by_key(|l| l.pubkey());
        $vec.dedup();
    };
}

fn check_fee_payer_balance(config: &Config, required_balance: u64) -> Result<(), Error> {
    let balance = config.rpc_client.get_balance(&config.fee_payer.pubkey())?;
    if balance < required_balance {
        Err(format!(
            "Fee payer, {}, has insufficient balance: {} required, {} available",
            config.fee_payer.pubkey(),
            lamports_to_sol(required_balance),
            lamports_to_sol(balance)
        )
        .into())
    } else {
        Ok(())
    }
}

fn get_program_accounts(
    config: &Config,
    account_type: AccountType,
    pubkey: &Pubkey,
) -> Result<Vec<(Pubkey, Account)>, ClientError> {
    config.rpc_client.get_program_accounts_with_config(
        &everlend_income_pools::id(),
        RpcProgramAccountsConfig {
            filters: Some(vec![
                // Account type
                RpcFilterType::Memcmp(Memcmp {
                    offset: 0,
                    bytes: MemcmpEncodedBytes::Base58(
                        bs58::encode([account_type as u8]).into_string(),
                    ),
                    encoding: Some(MemcmpEncoding::Binary),
                }),
                // Account parent
                RpcFilterType::Memcmp(Memcmp {
                    offset: 1,
                    bytes: MemcmpEncodedBytes::Base58(pubkey.to_string()),
                    encoding: Some(MemcmpEncoding::Binary),
                }),
            ]),
            account_config: RpcAccountInfoConfig {
                encoding: Some(UiAccountEncoding::Base64Zstd),
                ..RpcAccountInfoConfig::default()
            },
            ..RpcProgramAccountsConfig::default()
        },
    )
}

fn command_create_market(
    config: &Config,
    market_keypair: Option<Keypair>,
    general_pool_market_pubkey: &Pubkey,
) -> CommandResult {
    let market_keypair = market_keypair.unwrap_or_else(Keypair::new);

    println!("Income pool market: {}", market_keypair.pubkey());
    println!("General pool market: {}", market_keypair.pubkey());

    let market_balance = config
        .rpc_client
        .get_minimum_balance_for_rent_exemption(IncomePoolMarket::LEN)?;
    let total_rent_free_balances = market_balance;

    let mut tx = Transaction::new_with_payer(
        &[
            // Pool market account
            system_instruction::create_account(
                &config.fee_payer.pubkey(),
                &market_keypair.pubkey(),
                market_balance,
                IncomePoolMarket::LEN as u64,
                &everlend_income_pools::id(),
            ),
            // Initialize pool market account
            instruction::init_pool_market(
                &everlend_income_pools::id(),
                &market_keypair.pubkey(),
                &config.owner.pubkey(),
                general_pool_market_pubkey,
            ),
        ],
        Some(&config.fee_payer.pubkey()),
    );

    let (recent_blockhash, fee_calculator) = config.rpc_client.get_recent_blockhash()?;
    check_fee_payer_balance(
        config,
        total_rent_free_balances + fee_calculator.calculate_fee(tx.message()),
    )?;

    let mut signers = vec![
        config.fee_payer.as_ref(),
        config.owner.as_ref(),
        &market_keypair,
    ];

    unique_signers!(signers);
    tx.sign(&signers, recent_blockhash);

    Ok(Some(tx))
}

fn command_market_info(config: &Config, market_pubkey: &Pubkey) -> CommandResult {
    let market_account = config.rpc_client.get_account(market_pubkey)?;
    let market = IncomePoolMarket::unpack(&market_account.data)?;

    println!("{:#?}", market);

    println!("Pools:");
    let pools: Vec<(Pubkey, IncomePool)> =
        get_program_accounts(config, AccountType::IncomePool, market_pubkey)?
            .into_iter()
            .filter_map(
                |(address, account)| match IncomePool::unpack_unchecked(&account.data) {
                    Ok(pool) => Some((address, pool)),
                    _ => None,
                },
            )
            .collect();

    println!("{:#?}", pools);

    Ok(None)
}

fn command_create_pool(
    config: &Config,
    market_pubkey: &Pubkey,
    token_mint: &Pubkey,
) -> CommandResult {
    // Generate new accounts
    let token_account = Keypair::new();

    let (pool_pubkey, _) = everlend_income_pools::find_pool_program_address(
        &everlend_income_pools::id(),
        market_pubkey,
        token_mint,
    );

    println!("Pool: {}", &pool_pubkey);
    println!("Token mint: {}", &token_mint);
    println!("Token account: {}", &token_account.pubkey());
    println!("Market: {}", &market_pubkey);

    let token_account_balance = config
        .rpc_client
        .get_minimum_balance_for_rent_exemption(spl_token::state::Account::LEN)?;

    let total_rent_free_balances = token_account_balance;

    let mut tx = Transaction::new_with_payer(
        &[
            system_instruction::create_account(
                &config.fee_payer.pubkey(),
                &token_account.pubkey(),
                token_account_balance,
                spl_token::state::Account::LEN as u64,
                &spl_token::id(),
            ),
            instruction::create_pool(
                &everlend_income_pools::id(),
                market_pubkey,
                token_mint,
                &token_account.pubkey(),
                &config.owner.pubkey(),
            ),
        ],
        Some(&config.fee_payer.pubkey()),
    );

    let (recent_blockhash, fee_calculator) = config.rpc_client.get_recent_blockhash()?;
    check_fee_payer_balance(
        config,
        total_rent_free_balances + fee_calculator.calculate_fee(tx.message()),
    )?;

    let mut signers = vec![
        config.fee_payer.as_ref(),
        config.owner.as_ref(),
        &token_account,
    ];

    unique_signers!(signers);
    tx.sign(&signers, recent_blockhash);

    Ok(Some(tx))
}

fn command_pool_info(config: &Config, pool_pubkey: &Pubkey) -> CommandResult {
    let pool_account = config.rpc_client.get_account(pool_pubkey)?;
    let pool = IncomePool::unpack(&pool_account.data)?;

    println!("{:#?}", pool);

    Ok(None)
}

fn main() {
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
            SubCommand::with_name("create-market")
                .about("Create a new market")
                .arg(
                    Arg::with_name("market_keypair")
                        .long("keypair")
                        .validator(is_keypair_or_ask_keyword)
                        .value_name("PATH")
                        .takes_value(true)
                        .help("Market keypair [default: new keypair]"),
                )
                .arg(
                    Arg::with_name("general_market_pubkey")
                        .long("general-market")
                        .validator(is_pubkey)
                        .value_name("ADDRESS")
                        .takes_value(true)
                        .required(true)
                        .help("General market pubkey"),
                ),
        )
        .subcommand(
            SubCommand::with_name("market-info")
                .about("Print out market information")
                .arg(
                    Arg::with_name("market_pubkey")
                        .validator(is_pubkey)
                        .value_name("ADDRESS")
                        .takes_value(true)
                        .required(true)
                        .index(1)
                        .help("Market pubkey"),
                ),
        )
        .subcommand(
            SubCommand::with_name("create-pool")
                .about("Add a pool")
                .arg(
                    Arg::with_name("market_pubkey")
                        .long("market")
                        .validator(is_pubkey)
                        .value_name("ADDRESS")
                        .takes_value(true)
                        .required(true)
                        .help("Market pubkey"),
                )
                .arg(
                    Arg::with_name("token_mint")
                        .long("token")
                        .validator(is_pubkey)
                        .value_name("ADDRESS")
                        .takes_value(true)
                        .required(true)
                        .help("Mint for the token to be added to the pool"),
                ),
        )
        .subcommand(
            SubCommand::with_name("pool-info")
                .about("Print out pool information")
                .arg(
                    Arg::with_name("pool_pubkey")
                        .validator(is_pubkey)
                        .value_name("ADDRESS")
                        .takes_value(true)
                        .required(true)
                        .index(1)
                        .help("Pool pubkey"),
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
        .unwrap_or_else(|e| {
            eprintln!("error: {}", e);
            exit(1);
        });

        let fee_payer = signer_from_path(
            &matches,
            &cli_config.keypair_path,
            "fee_payer",
            &mut wallet_manager,
        )
        .unwrap_or_else(|e| {
            eprintln!("error: {}", e);
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
        ("create-market", Some(arg_matches)) => {
            let market_keypair = keypair_of(arg_matches, "market_keypair");
            let general_market_pubkey = pubkey_of(arg_matches, "general_market_pubkey").unwrap();
            command_create_market(&config, market_keypair, &general_market_pubkey)
        }
        ("market-info", Some(arg_matches)) => {
            let market_pubkey = pubkey_of(arg_matches, "market_pubkey").unwrap();
            command_market_info(&config, &market_pubkey)
        }
        ("create-pool", Some(arg_matches)) => {
            let market_pubkey = pubkey_of(arg_matches, "market_pubkey").unwrap();
            let token_mint = pubkey_of(arg_matches, "token_mint").unwrap();
            command_create_pool(&config, &market_pubkey, &token_mint)
        }
        ("pool-info", Some(arg_matches)) => {
            let pool_pubkey = pubkey_of(arg_matches, "pool_pubkey").unwrap();
            command_pool_info(&config, &pool_pubkey)
        }
        _ => unreachable!(),
    }
    .and_then(|tx| {
        if let Some(tx) = tx {
            let signature = config
                .rpc_client
                .send_and_confirm_transaction_with_spinner(&tx)?;
            println!("Signature: {}", signature);
        }
        Ok(())
    })
    .map_err(|err| {
        eprintln!("{}", err);
        exit(1);
    });
}
