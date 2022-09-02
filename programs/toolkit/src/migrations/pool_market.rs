use crate::{
    command_run_migrate_pool_market,
    utils::{arg_keypair, Config},
    ToolkitCommand,
};
use clap::{Arg, ArgMatches};
use solana_clap_utils::{input_parsers::keypair_of, input_validators::is_keypair};

const ARG_POOL_MARKET: &str = "pool-market";

pub struct MigratePoolMarketCommand;

impl<'a> ToolkitCommand<'a> for MigratePoolMarketCommand {
    fn get_name(&self) -> &'a str {
        return "pool-market";
    }

    fn get_description(&self) -> &'a str {
        return "migrate pool market";
    }

    fn get_args(&self) -> Vec<Arg<'a, 'a>> {
        return vec![arg_keypair(ARG_POOL_MARKET, true)];
    }

    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>> {
        return vec![];
    }

    fn handle(&self, config: &Config, arg_matches: Option<&ArgMatches>) -> anyhow::Result<()> {
        let arg_matches = arg_matches.unwrap();

        let pool_market_keypair = keypair_of(arg_matches, ARG_POOL_MARKET).unwrap();

        println!("Started pool market migration");
        command_run_migrate_pool_market(&config, pool_market_keypair)
    }
}
