use crate::{print_commands, utils::Config, ToolkitCommand};
use clap::{Arg, ArgMatches};

use super::{
    MigrateDepositorCommand, MigrateGeneralPoolCommand, MigrateLiquidityOracleCommand,
    MigratePoolMarketCommand, MigrateRebalancingCommand, MigrateRewardsPoolCommand,
    MigrateRewardsRootCommand,
};

#[derive(Clone, Copy)]
pub struct MigrationsCommand;

impl<'a> ToolkitCommand<'a> for MigrationsCommand {
    fn get_name(&self) -> &'a str {
        "migrations"
    }

    fn get_description(&self) -> &'a str {
        "run migration tools"
    }

    fn get_args(&self) -> Vec<Arg<'a, 'a>> {
        vec![]
    }

    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>> {
        vec![
            Box::new(MigrateGeneralPoolCommand),
            Box::new(MigrateDepositorCommand),
            Box::new(MigratePoolMarketCommand),
            Box::new(MigrateLiquidityOracleCommand),
            Box::new(MigrateRewardsRootCommand),
            Box::new(MigrateRewardsPoolCommand),
            Box::new(MigrateRebalancingCommand),
        ]
    }

    fn handle(&self, config: &Config, arg_matches: Option<&ArgMatches>) -> anyhow::Result<()> {
        let (cmd_name, arg_matches) = arg_matches.unwrap().subcommand();
        if cmd_name.is_empty() {
            print_commands(self);
            return Ok(());
        }

        let cmd = self
            .get_subcommands()
            .into_iter()
            .find(|x| x.get_name() == cmd_name)
            .unwrap();

        cmd.handle(config, arg_matches)
    }
}
