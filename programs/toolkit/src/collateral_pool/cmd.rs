use super::{
    CreatePoolCommand, CreatePoolWithdrawAuthorityCommand, CreatePoolsCommand,
    InitPoolMarketCommand,
};
use crate::{Config, ToolkitCommand};
use clap::{Arg, ArgMatches};

pub struct CollateralPoolCommand;

impl<'a> ToolkitCommand<'a> for CollateralPoolCommand {
    fn get_name(&self) -> &'a str {
        "collateral-pool"
    }

    fn get_description(&self) -> &'a str {
        "Collateral pool tools"
    }

    fn get_args(&self) -> Vec<Arg<'a, 'a>> {
        vec![]
    }

    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>> {
        vec![
            Box::new(CreatePoolCommand),
            Box::new(CreatePoolWithdrawAuthorityCommand),
            Box::new(CreatePoolsCommand),
            Box::new(InitPoolMarketCommand),
        ]
    }

    fn handle(&self, config: &Config, arg_matches: Option<&ArgMatches>) -> anyhow::Result<()> {
        let (cmd_name, arg_matches) = arg_matches.unwrap().subcommand();
        println!("{}", cmd_name);

        let cmd = self
            .get_subcommands()
            .into_iter()
            .find(|x| x.get_name() == cmd_name)
            .unwrap();

        cmd.handle(config, arg_matches)
    }
}
