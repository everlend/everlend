use crate::{
    utils::{arg_keypair, Config},
    ToolkitCommand,
};
use clap::{Arg, ArgMatches};
use solana_clap_utils::input_parsers::keypair_of;

const ARG_REGISTRY: &str = "registry";

pub struct InitRegistryCommand;

impl<'a> ToolkitCommand<'a> for InitRegistryCommand {
    fn get_name(&self) -> &'a str {
        return "init";
    }

    fn get_description(&self) -> &'a str {
        return "init registry";
    }

    fn get_args(&self) -> Vec<Arg<'a, 'a>> {
        return vec![arg_keypair(ARG_REGISTRY, true)];
    }

    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>> {
        return vec![];
    }

    fn handle(&self, config: &Config, arg_matches: Option<&ArgMatches>) -> anyhow::Result<()> {
        let arg_matches = arg_matches.unwrap();

        println!("Started Depositor migration");

        let keypair = keypair_of(arg_matches, ARG_REGISTRY).unwrap();
        // command_create_registry(&config, Some(keypair))?;
        // TODO: move this command inside this

        Ok(())
    }
}
