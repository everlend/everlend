use crate::helpers::{migrate_depositor, migrate_rebalancing};
use crate::{utils::Config, ToolkitCommand};
use clap::{Arg, ArgMatches};
use crate::utils::arg_multiple;
use crate::utils::get_asset_maps;
pub struct MigrateDepositorCommand;

const ARG_MINTS: &str = "mints";

impl<'a> ToolkitCommand<'a> for MigrateDepositorCommand {
    fn get_name(&self) -> &'a str {
        "depositor"
    }

    fn get_description(&self) -> &'a str {
        "Migrate Rebalnce account"
    }

    fn get_args(&self) -> Vec<Arg<'a, 'a>> {
        vec![
            arg_multiple(ARG_MINTS, true).short("m"),
        ]
    }

    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>> {
        vec![]
    }

    fn handle(&self, config: &Config, arg_matches: Option<&ArgMatches>) -> anyhow::Result<()> {
        let arg_matches = arg_matches.unwrap();
        let required_mints: Vec<_> = arg_matches.values_of(ARG_MINTS).unwrap().collect();

        println!("Started Depositor accounts migration");
        let initialized_accounts = config.get_initialized_accounts();

        for key in required_mints {
           let token_accounts =  initialized_accounts.token_accounts.get(key).unwrap();

            migrate_depositor(
                config,
                &initialized_accounts.depositor,
                &initialized_accounts.registry,
                &token_accounts.mint,
                &initialized_accounts.general_pool_market,
                &token_accounts.general_pool_token_account,
            )?;
        }


        println!("Migration of Rebalancing accounts finished",);

        Ok(())
    }
}
