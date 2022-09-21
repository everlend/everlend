use crate::helpers::migrate_depositor;
use crate::{utils::Config, ToolkitCommand};
use clap::{Arg, ArgMatches};

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
        println!("Started Depositor migration");
        let acc = config.get_initialized_accounts();

        for token in acc.token_accounts {
            migrate_depositor(config, &acc.depositor, &acc.registry, &token.1.mint)?;
        }

        Ok(())
    }
}
