use clap::{Arg, ArgMatches};
use crate::{Config, ToolkitCommand};
use super::{CreatePoolCommand, CreatePoolWithdrawAuthorityCommand, CreateCollateralPoolsCommand, InitPoolMarketCommand};

pub struct CollateralPoolCommand;

impl<'a> ToolkitCommand<'a> for CollateralPoolCommand {
    fn get_name(&self) -> &'a str {
        return "collateral-pool";
    }

    fn get_description(&self) -> &'a str {
        return "Collateral pool tools"
    }

    fn get_args(&self) -> Vec<Arg<'a, 'a>> {
        return vec![];
    }

    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>> {
        return vec![
            Box::new(CreatePoolCommand),
            Box::new(CreatePoolWithdrawAuthorityCommand),
            Box::new(CreateCollateralPoolsCommand),
            Box::new(InitPoolMarketCommand),
        ]
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