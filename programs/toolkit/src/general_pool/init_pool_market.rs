use crate::helpers::create_general_pool_market;
use crate::utils::arg_pubkey;
use crate::{arg_keypair, Config, ToolkitCommand};
use clap::{Arg, ArgMatches};
use solana_clap_utils::input_parsers::{keypair_of, pubkey_of};

const ARG_KEYPAIR: &str = "keypair";
const ARG_REGISTRY: &str = "registry";

#[derive(Clone, Copy)]
pub struct InitPoolMarketCommand;

impl<'a> ToolkitCommand<'a> for InitPoolMarketCommand {
    fn get_name(&self) -> &'a str {
        "init-pool-market"
    }

    fn get_description(&self) -> &'a str {
        "Init a new general pool market"
    }

    fn get_args(&self) -> Vec<Arg<'a, 'a>> {
        vec![
            arg_keypair(ARG_KEYPAIR, false),
            arg_pubkey(ARG_REGISTRY, true).help("Registry pubkey"),
        ]
    }

    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>> {
        vec![]
    }

    fn handle(&self, config: &Config, arg_matches: Option<&ArgMatches>) -> anyhow::Result<()> {
        let arg_matches = arg_matches.unwrap();

        let keypair = keypair_of(arg_matches, ARG_KEYPAIR);
        let registry_pubkey = pubkey_of(arg_matches, ARG_REGISTRY).unwrap();

        let mut initialiazed_accounts = config.get_initialized_accounts();

        let general_pool_market_pubkey =
            create_general_pool_market(config, keypair, &registry_pubkey)?;

        initialiazed_accounts.general_pool_market = general_pool_market_pubkey;

        initialiazed_accounts
            .save(&format!("accounts.{}.yaml", config.network))
            .unwrap();

        Ok(())
    }
}
