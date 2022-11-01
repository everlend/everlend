use crate::helpers::migrate_rebalancing;
use crate::utils::get_program_accounts;
use crate::{utils::Config, ToolkitCommand};
use clap::{Arg, ArgMatches};
use everlend_depositor::state::{AccountType, DeprecatedRebalancing};
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;

pub struct MigrateDepositorCommand;

impl<'a> ToolkitCommand<'a> for MigrateDepositorCommand {
    fn get_name(&self) -> &'a str {
        "depositor"
    }

    fn get_description(&self) -> &'a str {
        "Migrate Rebalnce account"
    }

    fn get_args(&self) -> Vec<Arg<'a, 'a>> {
        vec![]
    }

    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>> {
        vec![]
    }

    fn handle(&self, config: &Config, _arg_matches: Option<&ArgMatches>) -> anyhow::Result<()> {
        println!("Started Rebalancing accounts migration");
        let acc = config.get_initialized_accounts();

        let rebalancing_accounts: Vec<Pubkey> = get_program_accounts(
            config,
            &everlend_depositor::id(),
            AccountType::Rebalancing as u8,
            &acc.depositor,
        )?
        .into_iter()
        .filter_map(
            |(pk, account)| match DeprecatedRebalancing::unpack_unchecked(&account.data) {
                Ok(_) => Some(pk),
                _ => None,
            },
        )
        .collect();

        println!("Migrating depositor rebalancing accounts:");
        println!("{:#?}", rebalancing_accounts);

        migrate_rebalancing(config, &acc.depositor, &acc.registry, rebalancing_accounts)?;

        Ok(())
    }
}
