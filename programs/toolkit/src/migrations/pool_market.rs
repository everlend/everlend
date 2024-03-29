use crate::helpers::{close_pool_market_account, create_general_pool_market};
use crate::{
    utils::{arg_keypair, Config},
    ToolkitCommand,
};
use clap::{Arg, ArgMatches};
use solana_clap_utils::input_parsers::keypair_of;

const ARG_POOL_MARKET: &str = "pool-market";

pub struct MigratePoolMarketCommand;

impl<'a> ToolkitCommand<'a> for MigratePoolMarketCommand {
    fn get_name(&self) -> &'a str {
        "pool-market"
    }

    fn get_description(&self) -> &'a str {
        "migrate pool market"
    }

    fn get_args(&self) -> Vec<Arg<'a, 'a>> {
        vec![arg_keypair(ARG_POOL_MARKET, true)]
    }

    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>> {
        vec![]
    }

    fn handle(&self, config: &Config, arg_matches: Option<&ArgMatches>) -> anyhow::Result<()> {
        let arg_matches = arg_matches.unwrap();
        let initialized_accounts = config.get_initialized_accounts();
        let pool_market_keypair = keypair_of(arg_matches, ARG_POOL_MARKET).unwrap();

        println!("Started pool market migration");
        println!("Close general pool market");
        println!(
            "pool market id: {}",
            &initialized_accounts.general_pool_market
        );
        close_pool_market_account(config, &initialized_accounts.general_pool_market)?;
        println!("Closed general pool market");

        println!("Create general pool market");
        create_general_pool_market(
            config,
            Some(pool_market_keypair),
            &initialized_accounts.registry,
        )?;
        println!("Finished!");

        Ok(())
    }
}
