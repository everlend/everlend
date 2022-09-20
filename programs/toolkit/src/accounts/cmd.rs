use crate::{utils::Config, ToolkitCommand};

use clap::{Arg, ArgMatches};

use super::{
    AddReserveLiquidityCommand, CreateAccountsCommand, CreateTokenAccountsCommand, GetTokenCommand,
    InfoCommand, InfoReserveLiquidityCommand, InitQuarryMiningAccountsCommand,
    SaveLarixAccountsCommand, SaveQuarryAccountsCommand,
};

#[derive(Clone, Copy)]
pub struct AccountsCommand;

impl<'a> ToolkitCommand<'a> for AccountsCommand {
    fn get_name(&self) -> &'a str {
        "accounts"
    }

    fn get_description(&self) -> &'a str {
        "Accounts tools"
    }

    fn get_args(&self) -> Vec<Arg<'a, 'a>> {
        vec![]
    }

    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>> {
        vec![
            Box::new(AddReserveLiquidityCommand),
            Box::new(CreateAccountsCommand),
            Box::new(CreateTokenAccountsCommand),
            Box::new(GetTokenCommand),
            Box::new(InfoCommand),
            Box::new(InfoReserveLiquidityCommand),
            Box::new(InitQuarryMiningAccountsCommand),
            Box::new(SaveLarixAccountsCommand),
            Box::new(SaveQuarryAccountsCommand),
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
