use crate::{Config, ToolkitCommand};
use clap::{Arg, ArgMatches};
use everlend_depositor::find_rebalancing_program_address;
use everlend_depositor::state::Rebalancing;
use everlend_general_pool::find_pool_program_address;
use everlend_general_pool::state::Pool;

#[derive(Clone, Copy)]
pub struct DumpAccountsCommand;

impl<'a> ToolkitCommand<'a> for DumpAccountsCommand {
    fn get_name(&self) -> &'a str {
        "dump-accounts"
    }

    fn get_description(&self) -> &'a str {
        "Dump rebalancing accounts"
    }

    fn get_args(&self) -> Vec<Arg<'a, 'a>> {
        vec![]
    }

    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>> {
        vec![]
    }

    fn handle(&self, config: &Config, _arg_matches: Option<&ArgMatches>) -> anyhow::Result<()> {
        let acc = config.get_initialized_accounts();

        for pair in acc.token_accounts.into_iter() {
            let (rebalancing_pubkey, _) = find_rebalancing_program_address(
                &everlend_depositor::id(),
                &acc.depositor,
                &pair.1.mint,
            );

            let (pool_pubkey, _) = find_pool_program_address(
                &everlend_general_pool::id(),
                &acc.general_pool_market,
                &pair.1.mint,
            );

            let oracle: Rebalancing = config.get_account_unpack(&rebalancing_pubkey)?;
            let pool: Pool = config.get_account_unpack(&pool_pubkey)?;

            println!(
                "{} distributed_liquidity: {} amount_to_distribute: {} - total_amount_borrowed: {}. Diff {}",
                pair.0,
                oracle.distributed_liquidity,
                oracle.amount_to_distribute,
                pool.total_amount_borrowed,
                oracle.amount_to_distribute.saturating_sub(pool.total_amount_borrowed),
            );
        }

        Ok(())
    }
}
