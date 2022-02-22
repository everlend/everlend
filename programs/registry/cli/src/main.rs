use clap::{
    crate_description, crate_name, crate_version, value_t, App, AppSettings, Arg, SubCommand,
};

use everlend_registry::{
    instruction,
    state::{Registry, RegistryConfig, SetRegistryConfigParams, TOTAL_DISTRIBUTIONS},
};
use everlend_utils::integrations;
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
use std::{env, process::exit, str::FromStr};

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

fn command_create_registry(config: &Config, registry_keypair: Option<Keypair>) -> CommandResult {
    let registry_keypair = registry_keypair.unwrap_or_else(Keypair::new);

    // let (registry_config_pubkey, _) =
    //     &everlend_registry::find_config_program_address(&everlend_registry::id(), registry);

    // println!("Registry config: {}", registry_config_pubkey);
    println!("Registry: {}", registry_keypair.pubkey());

    let registry_balance = config
        .rpc_client
        .get_minimum_balance_for_rent_exemption(Registry::LEN)?;

    let total_rent_free_balances = registry_balance;

    let mut tx = Transaction::new_with_payer(
        &[
            // Registry account
            system_instruction::create_account(
                &config.fee_payer.pubkey(),
                &registry_keypair.pubkey(),
                registry_balance,
                Registry::LEN as u64,
                &everlend_registry::id(),
            ),
            // Initialize registry account
            instruction::init(
                &everlend_registry::id(),
                &registry_keypair.pubkey(),
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
        &registry_keypair,
    ];

    unique_signers!(signers);
    tx.sign(&signers, recent_blockhash);

    Ok(Some(tx))
}

fn command_registry_info(config: &Config, registry_pubkey: &Pubkey) -> CommandResult {
    let (registry_config_pubkey, _) =
        &everlend_registry::find_config_program_address(&everlend_registry::id(), registry_pubkey);

    let registry_config_account = config.rpc_client.get_account(registry_config_pubkey)?;
    let registry_config = RegistryConfig::unpack(&registry_config_account.data)?;

    println!("{:#?}", registry_config);

    Ok(None)
}

fn command_set_registry_config(
    config: &Config,
    registry_pubkey: &Pubkey,
    params: SetRegistryConfigParams,
) -> CommandResult {
    let (registry_config_pubkey, _) =
        &everlend_registry::find_config_program_address(&everlend_registry::id(), registry_pubkey);

    println!("Registry: {}", &registry_pubkey);
    println!("Registry config: {}", &registry_config_pubkey);

    let mut tx = Transaction::new_with_payer(
        &[instruction::set_registry_config(
            &everlend_registry::id(),
            registry_pubkey,
            &config.owner.pubkey(),
            params,
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
            SubCommand::with_name("create-registry")
                .about("Create a new registry")
                .arg(
                    Arg::with_name("registry_keypair")
                        .long("keypair")
                        .validator(is_keypair_or_ask_keyword)
                        .value_name("PATH")
                        .takes_value(true)
                        .help("Registry keypair [default: new keypair]"),
                ),
        )
        .subcommand(
            SubCommand::with_name("registry-info")
                .about("Print out registry information")
                .arg(
                    Arg::with_name("registry_pubkey")
                        .validator(is_pubkey)
                        .value_name("ADDRESS")
                        .takes_value(true)
                        .required(true)
                        .index(1)
                        .help("Registry pubkey"),
                ),
        )
        .subcommand(
            SubCommand::with_name("set-registry-config")
                .about("Set a registry config")
                .arg(
                    Arg::with_name("registry_pubkey")
                        .long("registry")
                        .validator(is_pubkey)
                        .value_name("ADDRESS")
                        .takes_value(true)
                        .required(true)
                        .help("Registry pubkey"),
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
        ("create-registry", Some(arg_matches)) => {
            let registry_keypair = keypair_of(arg_matches, "registry_keypair");

            command_create_registry(&config, registry_keypair)
        }
        ("registry-info", Some(arg_matches)) => {
            let registry_pubkey = pubkey_of(arg_matches, "registry_pubkey").unwrap();

            command_registry_info(&config, &registry_pubkey)
        }
        ("set-registry-config", Some(arg_matches)) => {
            let registry_pubkey = pubkey_of(arg_matches, "registry_pubkey").unwrap();

            let port_finance_program_id =
                Pubkey::from_str(integrations::PORT_FINANCE_PROGRAM_ID).unwrap();
            let larix_program_id = Pubkey::from_str(integrations::LARIX_PROGRAM_ID).unwrap();

            // TODO: fix it on yaml
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

            command_set_registry_config(&config, &registry_pubkey, registry_config)
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
