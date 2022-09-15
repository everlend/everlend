use crate::helpers::{get_general_pool_market, get_withdrawal_requests};
use crate::{Config, ToolkitCommand};
use clap::{Arg, ArgMatches};
use everlend_depositor::find_rebalancing_program_address;
use everlend_depositor::state::Rebalancing;

#[derive(Clone, Copy)]
pub struct InfoCommand;

impl<'a> ToolkitCommand<'a> for InfoCommand {
    fn get_name(&self) -> &'a str {
        "info"
    }

    fn get_description(&self) -> &'a str {
        "Print out env information"
    }

    fn get_args(&self) -> Vec<Arg<'a, 'a>> {
        vec![]
    }

    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>> {
        vec![]
    }

    fn handle(&self, config: &Config, _arg_matches: Option<&ArgMatches>) -> anyhow::Result<()> {
        let default_accounts = config.get_default_accounts();

        println!("fee_payer: {:?}", config.initialized_accounts.payer);
        println!("default_accounts = {:#?}", default_accounts);
        println!("{:#?}", config.initialized_accounts);

        println!(
            "{:#?}",
            get_general_pool_market(config, &config.initialized_accounts.general_pool_market)?
        );

        for (_, token_accounts) in config.initialized_accounts.token_accounts.iter() {
            println!("mint = {:?}", token_accounts.mint);
            let (withdraw_requests_pubkey, withdraw_requests) = get_withdrawal_requests(
                config,
                &config.initialized_accounts.general_pool_market,
                &token_accounts.mint,
            )?;
            println!("{:#?}", (withdraw_requests_pubkey, &withdraw_requests));

            let (rebalancing_pubkey, _) = find_rebalancing_program_address(
                &everlend_depositor::id(),
                &config.initialized_accounts.depositor,
                &token_accounts.mint,
            );

            let rebalancing = config.get_account_unpack::<Rebalancing>(&rebalancing_pubkey)?;
            println!("{:#?}", (rebalancing_pubkey, rebalancing));
        }

        Ok(())
    }
}
