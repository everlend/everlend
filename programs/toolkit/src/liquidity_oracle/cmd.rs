use crate::{utils::Config, ToolkitCommand};
use clap::{Arg, ArgMatches};
use super::{CreateLiquidityOracleCommand, UpdateAuthorityCommand};

#[derive(Clone, Copy)]
pub struct LiquidityOracleCommand;

impl<'a> ToolkitCommand<'a> for LiquidityOracleCommand {
    fn get_name(&self) -> &'a str {
        return "liquidity-oracle";
    }

    fn get_description(&self) -> &'a str {
        return "Liquidity Oracle tools";
    }

    fn get_args(&self) -> Vec<Arg<'a, 'a>> {
        return vec![];
    }

    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>> {
        return vec![
            Box::new(CreateLiquidityOracleCommand),
            Box::new(UpdateAuthorityCommand),
        ];
    }

    fn handle(&self, config: &Config, arg_matches: Option<&ArgMatches>) -> anyhow::Result<()> {
        let (cmd_name, _) = arg_matches.unwrap().subcommand();
        println!("{}", cmd_name);

        let cmd = self
            .get_subcommands()
            .into_iter()
            .find(|x| x.get_name() == cmd_name)
            .unwrap();

        cmd.handle(config, arg_matches)
    }
}
