use super::{
    ApproveCommand, CreateMultisigCommand, ExecuteCommand, InfoCommand, ProposeUpgradeCommand,
};
use crate::{print_commands, Config, ToolkitCommand};
use clap::{Arg, ArgMatches};

#[derive(Clone, Copy)]
pub struct MultisigCommand;

impl<'a> ToolkitCommand<'a> for MultisigCommand {
    fn get_name(&self) -> &'a str {
        "multisig"
    }

    fn get_description(&self) -> &'a str {
        "Multisig tools"
    }

    fn get_args(&self) -> Vec<Arg<'a, 'a>> {
        vec![]
    }

    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>> {
        vec![
            Box::new(CreateMultisigCommand),
            Box::new(ProposeUpgradeCommand),
            Box::new(ApproveCommand),
            Box::new(ExecuteCommand),
            Box::new(InfoCommand),
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
