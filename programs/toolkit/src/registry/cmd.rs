use crate::{print_commands, utils::Config, ToolkitCommand};
use clap::{Arg, ArgMatches};

use super::{InitRegistryCommand, SetRegistryCommand};

#[derive(Clone, Copy)]
pub struct RegistryCommand;

impl<'a> ToolkitCommand<'a> for RegistryCommand {
    fn get_name(&self) -> &'a str {
        "registry"
    }

    fn get_description(&self) -> &'a str {
        "registry tools"
    }

    fn get_args(&self) -> Vec<Arg<'a, 'a>> {
        vec![]
    }

    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>> {
        vec![Box::new(InitRegistryCommand), Box::new(SetRegistryCommand)]
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
