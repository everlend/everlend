use crate::utils::arg_pubkey;
use crate::{Config, ToolkitCommand};
use clap::{Arg, ArgMatches};
use everlend_depositor::find_rebalancing_program_address;
use everlend_depositor::state::Rebalancing;
use solana_clap_utils::input_parsers::pubkey_of;

const ARG_MINT: &str = "mint";

#[derive(Clone, Copy)]
pub struct GetRebalancingAccountCommand;

impl<'a> ToolkitCommand<'a> for GetRebalancingAccountCommand {
    fn get_name(&self) -> &'a str {
        "get-rebalancing-account"
    }

    fn get_description(&self) -> &'a str {
        "Get rebalancing account"
    }

    fn get_args(&self) -> Vec<Arg<'a, 'a>> {
        vec![arg_pubkey(ARG_MINT, true).help("Token mint")]
    }

    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>> {
        vec![]
    }

    fn handle(&self, config: &Config, arg_matches: Option<&ArgMatches>) -> anyhow::Result<()> {
        let arg_matches = arg_matches.unwrap();
        let mint = pubkey_of(arg_matches, ARG_MINT).unwrap();
        let acc = config.get_initialized_accounts();

        let (rebalancing_pubkey, _) =
            find_rebalancing_program_address(&everlend_depositor::id(), &acc.depositor, &mint);

        let oracle: Rebalancing = config.get_account_unpack(&rebalancing_pubkey)?;
        println!("{:#?}", oracle);

        Ok(())
    }
}
