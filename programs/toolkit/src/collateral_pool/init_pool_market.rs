use crate::helpers::create_collateral_market;
use crate::utils::arg;
use crate::{arg_keypair, Config, ToolkitCommand};
use clap::{Arg, ArgMatches};
use solana_clap_utils::input_parsers::{keypair_of, value_of};

const ARG_MONEY_MARKET: &str = "money-market";
const ARG_KEYPAIR: &str = "keypair";

#[derive(Clone, Copy)]
pub struct InitPoolMarketCommand;

impl<'a> ToolkitCommand<'a> for InitPoolMarketCommand {
    fn get_name(&self) -> &'a str {
        "init-pool-market"
    }

    fn get_description(&self) -> &'a str {
        "Init a new collateral pool market"
    }

    fn get_args(&self) -> Vec<Arg<'a, 'a>> {
        vec![
            arg(ARG_MONEY_MARKET, true)
                .value_name("NUMBER")
                .help("Money market index"),
            arg_keypair(ARG_KEYPAIR, false),
        ]
    }

    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>> {
        vec![]
    }

    fn handle(&self, config: &Config, arg_matches: Option<&ArgMatches>) -> anyhow::Result<()> {
        let arg_matches = arg_matches.unwrap();

        let keypair = keypair_of(arg_matches, ARG_KEYPAIR);
        let money_market = value_of::<usize>(arg_matches, ARG_MONEY_MARKET).unwrap();

        let mut initialiazed_accounts = config.get_initialized_accounts();

        let mm_pool_market_pubkey = create_collateral_market(config, keypair)?;
        initialiazed_accounts.collateral_pool_markets[money_market as usize] =
            mm_pool_market_pubkey;

        initialiazed_accounts
            .save(config.accounts_path.as_str())
            .unwrap();

        Ok(())
    }
}
