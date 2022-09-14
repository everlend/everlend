use crate::{utils::Config, ToolkitCommand};
use clap::{Arg, ArgMatches};

use super::InitRegistryCommand;

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
        vec![Box::new(InitRegistryCommand)]
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
