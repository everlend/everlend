use clap::{
    crate_description, crate_name, crate_version, value_t, App, AppSettings, Arg, SubCommand,
};
use everlend_ulp::{
    find_pool_borrow_authority_program_address, find_pool_program_address, id, instruction,
    state::{ui_bp_to_bp, AccountType, Pool, PoolBorrowAuthority, PoolMarket},
};
use solana_account_decoder::UiAccountEncoding;
use solana_clap_utils::{
    fee_payer::fee_payer_arg,
    input_parsers::{keypair_of, pubkey_of, value_of},
    input_validators::{
        is_amount, is_keypair, is_keypair_or_ask_keyword, is_pubkey, is_url_or_moniker,
    },
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
        &id(),
        RpcProgramAccountsConfig {
            filters: Some(vec![
                // Account type
                RpcFilterType::Memcmp(Memcmp {
                    offset: 0,
                    bytes: MemcmpEncodedBytes::Binary(
                        bs58::encode([account_type as u8]).into_string(),
                    ),
                    encoding: Some(MemcmpEncoding::Binary),
                }),
                // Account parent
                RpcFilterType::Memcmp(Memcmp {
                    offset: 1,
                    bytes: MemcmpEncodedBytes::Binary(pubkey.to_string()),
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

fn command_create_market(config: &Config, market_keypair: Option<Keypair>) -> CommandResult {
    let market_keypair = market_keypair.unwrap_or_else(Keypair::new);

    println!("Creating market {}", market_keypair.pubkey());

    let market_balance = config
        .rpc_client
        .get_minimum_balance_for_rent_exemption(PoolMarket::LEN)?;
    let total_rent_free_balances = market_balance;

    let mut tx = Transaction::new_with_payer(
        &[
            // Pool market account
            system_instruction::create_account(
                &config.fee_payer.pubkey(),
                &market_keypair.pubkey(),
                market_balance,
                PoolMarket::LEN as u64,
                &id(),
            ),
            // Initialize pool market account
            instruction::init_pool_market(&id(), &market_keypair.pubkey(), &config.owner.pubkey()),
        ],
        Some(&config.fee_payer.pubkey()),
    );

    let (recent_blockhash, fee_calculator) = config.rpc_client.get_recent_blockhash()?;
    check_fee_payer_balance(
        config,
        total_rent_free_balances + fee_calculator.calculate_fee(&tx.message()),
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
    let market_account = config.rpc_client.get_account(&market_pubkey)?;
    let market = PoolMarket::unpack(&market_account.data)?;

    println!("{:#?}", market);

    println!("Pools:");
    let pools: Vec<(Pubkey, Pool)> =
        get_program_accounts(config, AccountType::Pool, market_pubkey)?
            .into_iter()
            .filter_map(
                |(address, account)| match Pool::unpack_unchecked(&account.data) {
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
    let pool_mint = Keypair::new();

    let (pool_pubkey, _) = find_pool_program_address(&id(), market_pubkey, token_mint);

    println!("Pool: {}", &pool_pubkey);
    println!("Token mint: {}", &token_mint);
    println!("Token account: {}", &token_account.pubkey());
    println!("Pool mint: {}", &pool_mint.pubkey());
    println!("Market: {}", &market_pubkey);

    let token_account_balance = config
        .rpc_client
        .get_minimum_balance_for_rent_exemption(spl_token::state::Account::LEN)?;
    let pool_mint_balance = config
        .rpc_client
        .get_minimum_balance_for_rent_exemption(spl_token::state::Mint::LEN)?;

    let total_rent_free_balances = token_account_balance + pool_mint_balance;

    let mut tx = Transaction::new_with_payer(
        &[
            system_instruction::create_account(
                &config.fee_payer.pubkey(),
                &token_account.pubkey(),
                token_account_balance,
                spl_token::state::Account::LEN as u64,
                &spl_token::id(),
            ),
            system_instruction::create_account(
                &config.fee_payer.pubkey(),
                &pool_mint.pubkey(),
                pool_mint_balance,
                spl_token::state::Mint::LEN as u64,
                &spl_token::id(),
            ),
            instruction::create_pool(
                &id(),
                &market_pubkey,
                &token_mint,
                &token_account.pubkey(),
                &pool_mint.pubkey(),
                &config.owner.pubkey(),
            ),
        ],
        Some(&config.fee_payer.pubkey()),
    );

    let (recent_blockhash, fee_calculator) = config.rpc_client.get_recent_blockhash()?;
    check_fee_payer_balance(
        config,
        total_rent_free_balances + fee_calculator.calculate_fee(&tx.message()),
    )?;

    let mut signers = vec![
        config.fee_payer.as_ref(),
        config.owner.as_ref(),
        &token_account,
        &pool_mint,
    ];

    unique_signers!(signers);
    tx.sign(&signers, recent_blockhash);

    Ok(Some(tx))
}

fn command_pool_info(config: &Config, pool_pubkey: &Pubkey) -> CommandResult {
    let pool_account = config.rpc_client.get_account(&pool_pubkey)?;
    let pool = Pool::unpack(&pool_account.data)?;

    println!("{:#?}", pool);

    println!("Pool borrow authorities:");
    let pool_borrow_authorities: Vec<(Pubkey, PoolBorrowAuthority)> =
        get_program_accounts(config, AccountType::PoolBorrowAuthority, pool_pubkey)?
            .into_iter()
            .filter_map(|(address, account)| {
                match PoolBorrowAuthority::unpack_unchecked(&account.data) {
                    Ok(pool_borrow_authority) => Some((address, pool_borrow_authority)),
                    _ => None,
                }
            })
            .collect();

    println!("{:#?}", pool_borrow_authorities);

    Ok(None)
}

fn command_create_pool_borrow_authority(
    config: &Config,
    pool_pubkey: &Pubkey,
    borrow_authority: &Pubkey,
    ui_share_allowed: f64,
) -> CommandResult {
    let (pool_borrow_authority_pubkey, _) =
        find_pool_borrow_authority_program_address(&id(), pool_pubkey, borrow_authority);

    let pool_account = config.rpc_client.get_account(&pool_pubkey)?;
    let pool = Pool::unpack(&pool_account.data)?;

    println!("Pool borrow authority: {}", &pool_borrow_authority_pubkey);
    println!("Borrow authority: {}", &borrow_authority);
    println!("Pool: {}", &pool_pubkey);
    println!("Market: {}", &pool.pool_market);

    let share_allowed = ui_bp_to_bp(ui_share_allowed);

    let mut tx = Transaction::new_with_payer(
        &[instruction::create_pool_borrow_authority(
            &id(),
            &pool.pool_market,
            &pool_pubkey,
            &borrow_authority,
            &config.owner.pubkey(),
            share_allowed,
        )],
        Some(&config.fee_payer.pubkey()),
    );

    let (recent_blockhash, fee_calculator) = config.rpc_client.get_recent_blockhash()?;
    check_fee_payer_balance(config, fee_calculator.calculate_fee(&tx.message()))?;

    let mut signers = vec![config.fee_payer.as_ref(), config.owner.as_ref()];

    unique_signers!(signers);
    tx.sign(&signers, recent_blockhash);

    Ok(Some(tx))
}

fn command_update_pool_borrow_authority(
    config: &Config,
    pool_pubkey: &Pubkey,
    borrow_authority: &Pubkey,
    ui_share_allowed: f64,
) -> CommandResult {
    let (pool_borrow_authority_pubkey, _) =
        find_pool_borrow_authority_program_address(&id(), pool_pubkey, borrow_authority);

    println!("Pool borrow authority: {}", &pool_borrow_authority_pubkey);

    let share_allowed = ui_bp_to_bp(ui_share_allowed);

    let mut tx = Transaction::new_with_payer(
        &[instruction::update_pool_borrow_authority(
            &id(),
            &pool_pubkey,
            &borrow_authority,
            &config.owner.pubkey(),
            share_allowed,
        )],
        Some(&config.fee_payer.pubkey()),
    );

    let (recent_blockhash, fee_calculator) = config.rpc_client.get_recent_blockhash()?;
    check_fee_payer_balance(config, fee_calculator.calculate_fee(&tx.message()))?;

    let mut signers = vec![config.fee_payer.as_ref(), config.owner.as_ref()];

    unique_signers!(signers);
    tx.sign(&signers, recent_blockhash);

    Ok(Some(tx))
}

fn command_delete_pool_borrow_authority(
    config: &Config,
    pool_pubkey: &Pubkey,
    borrow_authority: &Pubkey,
) -> CommandResult {
    let (pool_borrow_authority_pubkey, _) =
        find_pool_borrow_authority_program_address(&id(), pool_pubkey, borrow_authority);

    println!("Pool borrow authority: {}", &pool_borrow_authority_pubkey);

    let mut tx = Transaction::new_with_payer(
        &[instruction::delete_pool_borrow_authority(
            &id(),
            &pool_pubkey,
            &borrow_authority,
            &config.fee_payer.pubkey(),
            &config.owner.pubkey(),
        )],
        Some(&config.fee_payer.pubkey()),
    );

    let (recent_blockhash, fee_calculator) = config.rpc_client.get_recent_blockhash()?;
    check_fee_payer_balance(config, fee_calculator.calculate_fee(&tx.message()))?;

    let mut signers = vec![config.fee_payer.as_ref(), config.owner.as_ref()];

    unique_signers!(signers);
    tx.sign(&signers, recent_blockhash);

    Ok(Some(tx))
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
                arg.default_value(&config_file)
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
        .subcommand(
            SubCommand::with_name("create-pool-borrow-authority")
                .about("Add a pool borrow authority")
                .arg(
                    Arg::with_name("pool_pubkey")
                        .long("pool")
                        .validator(is_pubkey)
                        .value_name("ADDRESS")
                        .takes_value(true)
                        .required(true)
                        .help("Pool pubkey"),
                )
                .arg(
                    Arg::with_name("borrow_authority")
                        .long("borrower")
                        .validator(is_pubkey)
                        .value_name("ADDRESS")
                        .takes_value(true)
                        .required(true)
                        .help("Borrow authority"),
                )
                .arg(
                    Arg::with_name("share_allowed")
                        .long("share")
                        .validator(is_amount)
                        .value_name("NUMBER")
                        .takes_value(true)
                        .default_value("1")
                        .help("Share allowed (for example, 0.15 for 15% of the pool)"),
                ),
        )
        .subcommand(
            SubCommand::with_name("update-pool-borrow-authority")
                .about("Update a pool borrow authority")
                .arg(
                    Arg::with_name("pool_pubkey")
                        .long("pool")
                        .validator(is_pubkey)
                        .value_name("ADDRESS")
                        .takes_value(true)
                        .required(true)
                        .help("Pool pubkey"),
                )
                .arg(
                    Arg::with_name("borrow_authority")
                        .long("borrower")
                        .validator(is_pubkey)
                        .value_name("ADDRESS")
                        .takes_value(true)
                        .required(true)
                        .help("Borrow authority"),
                )
                .arg(
                    Arg::with_name("share_allowed")
                        .long("share")
                        .validator(is_amount)
                        .value_name("NUMBER")
                        .takes_value(true)
                        .required(true)
                        .help("Share allowed (for example, 0.15 for 15% of the pool)"),
                ),
        )
        .subcommand(
            SubCommand::with_name("delete-pool-borrow-authority")
                .about("Delete a pool borrow authority")
                .arg(
                    Arg::with_name("pool_pubkey")
                        .long("pool")
                        .validator(is_pubkey)
                        .value_name("ADDRESS")
                        .takes_value(true)
                        .required(true)
                        .help("Pool pubkey"),
                )
                .arg(
                    Arg::with_name("borrow_authority")
                        .long("borrower")
                        .validator(is_pubkey)
                        .value_name("ADDRESS")
                        .takes_value(true)
                        .required(true)
                        .help("Borrow authority"),
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
            command_create_market(&config, market_keypair)
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
        ("create-pool-borrow-authority", Some(arg_matches)) => {
            let pool_pubkey = pubkey_of(arg_matches, "pool_pubkey").unwrap();
            let borrow_authority = pubkey_of(arg_matches, "borrow_authority").unwrap();
            let share_allowed = value_of::<f64>(arg_matches, "share_allowed").unwrap();
            command_create_pool_borrow_authority(
                &config,
                &pool_pubkey,
                &borrow_authority,
                share_allowed,
            )
        }
        ("update-pool-borrow-authority", Some(arg_matches)) => {
            let pool_pubkey = pubkey_of(arg_matches, "pool_pubkey").unwrap();
            let borrow_authority = pubkey_of(arg_matches, "borrow_authority").unwrap();
            let share_allowed = value_of::<f64>(arg_matches, "share_allowed").unwrap();
            command_update_pool_borrow_authority(
                &config,
                &pool_pubkey,
                &borrow_authority,
                share_allowed,
            )
        }
        ("delete-pool-borrow-authority", Some(arg_matches)) => {
            let pool_pubkey = pubkey_of(arg_matches, "pool_pubkey").unwrap();
            let borrow_authority = pubkey_of(arg_matches, "borrow_authority").unwrap();
            command_delete_pool_borrow_authority(&config, &pool_pubkey, &borrow_authority)
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
