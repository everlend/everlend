use crate::{general_pool, utils::Config, ToolkitCommand};
use clap::{Arg, ArgMatches};

pub struct MigrateGeneralPoolCommand;

impl<'a> ToolkitCommand<'a> for MigrateGeneralPoolCommand {
    fn get_name(&self) -> &'a str {
        return "general-pool";
    }

    fn get_description(&self) -> &'a str {
        return "migrate general pool";
    }

    fn get_args(&self) -> Vec<Arg<'a, 'a>> {
        return vec![];
    }

    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>> {
        return vec![];
    }

    fn handle(&self, config: &Config, _arg_matches: Option<&ArgMatches>) -> anyhow::Result<()> {
        // general_pool::migrate_general_pool_account(config)?;
        println!("Finished!");

        Ok(())
    }
}
