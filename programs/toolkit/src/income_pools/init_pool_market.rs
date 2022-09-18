use crate::helpers::create_income_pool_market;
use crate::{arg_keypair, Config, ToolkitCommand};
use clap::{Arg, ArgMatches};
use solana_clap_utils::input_parsers::keypair_of;

const ARG_KEYPAIR: &str = "keypair";

#[derive(Clone, Copy)]
pub struct InitPoolMarketCommand;

impl<'a> ToolkitCommand<'a> for InitPoolMarketCommand {
    fn get_name(&self) -> &'a str {
        "init-pool-market"
    }

    fn get_description(&self) -> &'a str {
        "Init a new income pool market"
    }

    fn get_args(&self) -> Vec<Arg<'a, 'a>> {
        vec![arg_keypair(ARG_KEYPAIR, false)]
    }

    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>> {
        vec![]
    }

    fn handle(&self, config: &Config, arg_matches: Option<&ArgMatches>) -> anyhow::Result<()> {
        let arg_matches = arg_matches.unwrap();
        let keypair = keypair_of(arg_matches, ARG_KEYPAIR);

        let mut initialiazed_accounts = config.get_initialized_accounts();

        let income_pool_market_pubkey =
            create_income_pool_market(config, keypair, &initialiazed_accounts.general_pool_market)?;

        initialiazed_accounts.income_pool_market = income_pool_market_pubkey;

        initialiazed_accounts
            .save(&format!("accounts.{}.yaml", config.network))
            .unwrap();

        Ok(())
    }
}
