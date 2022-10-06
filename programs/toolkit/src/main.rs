use std::path::PathBuf;
use std::process::exit;

use accounts::AccountsCommand;
use clap::{
    crate_description, crate_name, crate_version, value_t, App, AppSettings, Arg, ArgMatches,
    SubCommand,
};
use migrations::MigrationsCommand;
use regex::Regex;
use registry::RegistryCommand;
use rewards::RewardsCommand;
use root::{
    CreateTokenCommand, TestLarixMiningRawCommand, TestQuarryMiningRawCommand, UpdateManagerCommand,
};
use solana_clap_utils::{fee_payer::fee_payer_arg, keypair::signer_from_path};
use solana_client::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use utils::{arg_keypair, arg_path, Config};

use crate::accounts_config::InitializedAccounts;
use crate::collateral_pool::CollateralPoolCommand;
use crate::depositor::DepositorCommand;
use crate::general_pool::{CancelWithdrawRequestCommand, GeneralPoolCommand};
use crate::income_pools::IncomePoolCommand;
use crate::liquidity_oracle::LiquidityOracleCommand;
use crate::multisig::MultisigCommand;
use crate::root::TestCommand;
use crate::root::AnchorEncodeCommand;

mod accounts;
mod accounts_config;
mod collateral_pool;
mod depositor;
mod general_pool;
mod helpers;
mod income_pools;
mod liquidity_mining;
mod liquidity_oracle;
mod migrations;
mod multisig;
mod registry;
mod rewards;
mod root;
mod utils;

pub trait ToolkitCommand<'a> {
    fn get_name(&self) -> &'a str;
    fn get_description(&self) -> &'a str;
    fn get_args(&self) -> Vec<Arg<'a, 'a>>;
    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>>;
    fn handle(&self, config: &Config, arg_matches: Option<&ArgMatches>) -> anyhow::Result<()>;
}

fn print_commands(cmd: &dyn ToolkitCommand) {
    let cmds = cmd
        .get_subcommands()
        .into_iter()
        .map(|x| x.get_name())
        .collect::<Vec<&str>>()
        .join("\n\t");

    println!("Available commands:\n\t{}", cmds);
}

#[allow(clippy::borrowed_box)]
fn build_command<'a>(cmd: &Box<dyn ToolkitCommand<'a>>) -> App<'a, 'a> {
    let commands: Vec<App> = cmd
        .get_subcommands()
        .iter()
        .map(|a| -> App { build_command(a) })
        .collect();

    let x = SubCommand::with_name(cmd.get_name())
        .about(cmd.get_description())
        .args(&cmd.get_args())
        .subcommands(commands);

    x
}

const ARG_CONFIG: &str = "config";
const ARG_OWNER: &str = "owner";
const ARG_ACCOUNTS: &str = "accounts";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init()
}

fn init() -> anyhow::Result<()> {
    solana_logger::setup_with_default("solana=info");

    let commands: Vec<Box<dyn ToolkitCommand>> = vec![
        Box::new(RegistryCommand),
        Box::new(GeneralPoolCommand),
        Box::new(CollateralPoolCommand),
        Box::new(IncomePoolCommand),
        Box::new(LiquidityOracleCommand),
        Box::new(DepositorCommand),
        Box::new(TestLarixMiningRawCommand),
        Box::new(TestQuarryMiningRawCommand),
        Box::new(CancelWithdrawRequestCommand),
        Box::new(TestCommand),
        Box::new(MigrationsCommand),
        Box::new(AccountsCommand),
        Box::new(UpdateManagerCommand),
        Box::new(MultisigCommand),
        Box::new(CreateTokenCommand),
        Box::new(RewardsCommand),
        Box::new(AnchorEncodeCommand),
    ];

    let subcommands: Vec<App> = commands
        .iter()
        .map(|a| -> App { build_command(a) })
        .collect();

    let app = App::new(crate_name!())
        .about(crate_description!())
        .version(crate_version!())
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .arg({
            let arg = arg_path(ARG_CONFIG, false)
                .global(true)
                .short("C")
                .help("Configuration file to use");

            if let Some(ref config_file) = *solana_cli_config::CONFIG_FILE {
                arg.default_value(config_file)
            } else {
                arg
            }
        })
        .arg(
            arg_path(ARG_ACCOUNTS, false)
                .global(true)
                .help("Accounts file")
                .short("A"),
        )
        .arg(arg_keypair(ARG_OWNER, false).global(true).help(
            "Specify the token owner account. \
             This may be a keypair file, the ASK keyword. \
             Defaults to the client keypair.",
        ))
        .arg(fee_payer_arg().global(true))
        .subcommands(subcommands)
        .get_matches();

    let (cmd_name, arg_matches) = app.subcommand();

    let config = get_config(arg_matches.unwrap());

    let cmd = commands.iter().find(|x| x.get_name() == cmd_name).unwrap();

    cmd.handle(&config, arg_matches)?;

    Ok(())
}

fn get_config(matches: &ArgMatches) -> Config {
    let mut wallet_manager = None;
    let cli_config = if let Some(config_file) = matches.value_of(ARG_CONFIG) {
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
        matches,
        matches
            .value_of(ARG_OWNER)
            .unwrap_or(&cli_config.keypair_path),
        "owner",
        &mut wallet_manager,
    )
    .unwrap_or_else(|err| {
        eprintln!("Error: {}", err);
        exit(1);
    });

    let fee_payer = signer_from_path(
        matches,
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

    let accounts_path = matches
        .value_of(ARG_ACCOUNTS)
        .unwrap_or(&format!("accounts.{}.yaml", network))
        .to_string();

    Config {
        rpc_client: RpcClient::new_with_commitment(json_rpc_url, CommitmentConfig::confirmed()),
        owner,
        fee_payer,
        network,
        accounts_path,
    }
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
