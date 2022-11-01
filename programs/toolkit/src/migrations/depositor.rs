use crate::helpers::migrate_rebalancing;
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
        println!("Started Rebalancing accounts migration");
        migrate_rebalancing(config)?;
        println!("Migration of Rebalancing accounts finished",);

        Ok(())
    }
}
