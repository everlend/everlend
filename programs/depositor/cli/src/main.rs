use clap::{
    crate_description, crate_name, crate_version, value_t, App, AppSettings, Arg, SubCommand,
};
use everlend_depositor::{instruction, state::Depositor};

use solana_clap_utils::{
    fee_payer::fee_payer_arg,
    input_parsers::{keypair_of, pubkey_of},
    input_validators::{is_keypair, is_keypair_or_ask_keyword, is_pubkey, is_url_or_moniker},
    keypair::signer_from_path,
};
use solana_client::rpc_client::RpcClient;
use solana_program::{
    native_token::lamports_to_sol, program_pack::Pack, pubkey::Pubkey, system_instruction,
};
use solana_sdk::{
    commitment_config::CommitmentConfig, signature::Keypair, signer::Signer,
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

fn command_create_depositor(
    config: &Config,
    registry: &Pubkey,
    depositor_keypair: Option<Keypair>,
    general_pool_market_pubkey: &Pubkey,
    income_pool_market_pubkey: &Pubkey,
    liquidity_oracle_pubkey: &Pubkey,
) -> CommandResult {
    let depositor_keypair = depositor_keypair.unwrap_or_else(Keypair::new);

    let (registry_config_pubkey, _) =
        &everlend_registry::find_config_program_address(&everlend_registry::id(), registry);

    println!("Registry config: {}", registry_config_pubkey);
    println!("Depositor: {}", depositor_keypair.pubkey());
    println!("General pool market: {}", general_pool_market_pubkey);
    println!("Income pool market: {}", income_pool_market_pubkey);
    println!("Liquidity oracle: {}", liquidity_oracle_pubkey);

    let depositor_balance = config
        .rpc_client
        .get_minimum_balance_for_rent_exemption(Depositor::LEN)?;

    let total_rent_free_balances = depositor_balance;

    let mut tx = Transaction::new_with_payer(
        &[
            // Depositor account
            system_instruction::create_account(
                &config.fee_payer.pubkey(),
                &depositor_keypair.pubkey(),
                depositor_balance,
                Depositor::LEN as u64,
                &everlend_depositor::id(),
            ),
            // Initialize depositor account
            instruction::init(
                &everlend_depositor::id(),
                registry_config_pubkey,
                &depositor_keypair.pubkey(),
                general_pool_market_pubkey,
                income_pool_market_pubkey,
                liquidity_oracle_pubkey,
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
        &depositor_keypair,
    ];

    unique_signers!(signers);
    tx.sign(&signers, recent_blockhash);

    Ok(Some(tx))
}

fn command_depositor_info(config: &Config, depositor_pubkey: &Pubkey) -> CommandResult {
    let depositor_account = config.rpc_client.get_account(depositor_pubkey)?;
    let depositor = Depositor::unpack(&depositor_account.data)?;

    let (depositor_authority, _) =
        &everlend_utils::find_program_address(&everlend_depositor::id(), depositor_pubkey);

    println!("{:#?}", depositor);
    println!("Depositor authority: {:?}", depositor_authority);

    Ok(None)
}

fn command_create_transit(
    config: &Config,
    depositor_pubkey: &Pubkey,
    token_mint: &Pubkey,
) -> CommandResult {
    let (transit_pubkey, _) = everlend_depositor::find_transit_program_address(
        &everlend_depositor::id(),
        depositor_pubkey,
        token_mint,
    );

    println!("Transit: {}", &transit_pubkey);
    println!("Token mint: {}", &token_mint);
    println!("Depositor: {}", &depositor_pubkey);

    let mut tx = Transaction::new_with_payer(
        &[instruction::create_transit(
            &everlend_depositor::id(),
            depositor_pubkey,
            token_mint,
            &config.fee_payer.pubkey(),
        )],
        Some(&config.fee_payer.pubkey()),
    );

    let (recent_blockhash, _) = config.rpc_client.get_recent_blockhash()?;

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
            SubCommand::with_name("create-depositor")
                .about("Create a new depositor")
                .arg(
                    Arg::with_name("registry_pubkey")
                        .long("registry")
                        .validator(is_pubkey)
                        .value_name("ADDRESS")
                        .takes_value(true)
                        .required(true)
                        .help("Registry pubkey"),
                )
                .arg(
                    Arg::with_name("depositor_keypair")
                        .long("keypair")
                        .validator(is_keypair_or_ask_keyword)
                        .value_name("PATH")
                        .takes_value(true)
                        .help("Depositor keypair [default: new keypair]"),
                )
                .arg(
                    Arg::with_name("general_market_pubkey")
                        .long("general-market")
                        .validator(is_pubkey)
                        .value_name("ADDRESS")
                        .takes_value(true)
                        .required(true)
                        .help("General market pubkey"),
                )
                .arg(
                    Arg::with_name("income_market_pubkey")
                        .long("income-market")
                        .validator(is_pubkey)
                        .value_name("ADDRESS")
                        .takes_value(true)
                        .required(true)
                        .help("Income market pubkey"),
                )
                .arg(
                    Arg::with_name("liquidity_oracle_pubkey")
                        .long("liquidity-oracle")
                        .validator(is_pubkey)
                        .value_name("ADDRESS")
                        .takes_value(true)
                        .required(true)
                        .help("Liquidity oracle pubkey"),
                ),
        )
        .subcommand(
            SubCommand::with_name("depositor-info")
                .about("Print out depositor information")
                .arg(
                    Arg::with_name("depositor_pubkey")
                        .validator(is_pubkey)
                        .value_name("ADDRESS")
                        .takes_value(true)
                        .required(true)
                        .index(1)
                        .help("Depositor pubkey"),
                ),
        )
        .subcommand(
            SubCommand::with_name("create-transit")
                .about("Add a transit")
                .arg(
                    Arg::with_name("depositor_pubkey")
                        .long("depositor")
                        .validator(is_pubkey)
                        .value_name("ADDRESS")
                        .takes_value(true)
                        .required(true)
                        .help("Depositor pubkey"),
                )
                .arg(
                    Arg::with_name("token_mint")
                        .long("token")
                        .validator(is_pubkey)
                        .value_name("ADDRESS")
                        .takes_value(true)
                        .required(true)
                        .help("Mint for the token to be added to the transit"),
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
        ("create-depositor", Some(arg_matches)) => {
            let registry_pubkey = pubkey_of(arg_matches, "registry_pubkey").unwrap();
            let depositor_keypair = keypair_of(arg_matches, "depositor_keypair");
            let general_market_pubkey = pubkey_of(arg_matches, "general_market_pubkey").unwrap();
            let income_pool_market_pubkey = pubkey_of(arg_matches, "income_market_pubkey").unwrap();
            let liquidity_oracle_pubkey =
                pubkey_of(arg_matches, "liquidity_oracle_pubkey").unwrap();
            command_create_depositor(
                &config,
                &registry_pubkey,
                depositor_keypair,
                &general_market_pubkey,
                &income_pool_market_pubkey,
                &liquidity_oracle_pubkey,
            )
        }
        ("depositor-info", Some(arg_matches)) => {
            let depositor_pubkey = pubkey_of(arg_matches, "depositor_pubkey").unwrap();

            command_depositor_info(&config, &depositor_pubkey)
        }
        ("create-transit", Some(arg_matches)) => {
            let depositor_pubkey = pubkey_of(arg_matches, "depositor_pubkey").unwrap();
            let token_mint = pubkey_of(arg_matches, "token_mint").unwrap();
            command_create_transit(&config, &depositor_pubkey, &token_mint)
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
