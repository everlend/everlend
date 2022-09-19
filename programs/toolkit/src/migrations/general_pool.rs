use crate::helpers::migrate_general_pool_account;
use crate::{utils::Config, ToolkitCommand};
use clap::{Arg, ArgMatches};

pub struct MigrateGeneralPoolCommand;

impl<'a> ToolkitCommand<'a> for MigrateGeneralPoolCommand {
    fn get_name(&self) -> &'a str {
        "general-pool"
    }

    fn get_description(&self) -> &'a str {
        "migrate general pool"
    }

    fn get_args(&self) -> Vec<Arg<'a, 'a>> {
        vec![]
    }

    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>> {
        vec![]
    }

    fn handle(&self, config: &Config, _arg_matches: Option<&ArgMatches>) -> anyhow::Result<()> {
        println!("Migrate withdraw requests");
        migrate_general_pool_account(config)?;
        println!("Finished!");

        Ok(())
    }
}
