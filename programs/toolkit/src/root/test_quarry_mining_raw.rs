use crate::{
    utils::{arg, Config},
    ToolkitCommand,
};
use clap::{Arg, ArgMatches};

const ARG_TOKEN: &str = "token";

#[derive(Clone, Copy)]
pub struct TestQuarryMiningRawCommand;

impl<'a> ToolkitCommand<'a> for TestQuarryMiningRawCommand {
    fn get_name(&self) -> &'a str {
        return "test-quarry-mining-raw";
    }

    fn get_description(&self) -> &'a str {
        return "";
    }

    fn get_args(&self) -> Vec<Arg<'a, 'a>> {
        return vec![arg(ARG_TOKEN, true)];
    }

    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>> {
        return vec![];
    }

    fn handle(&self, config: &Config, arg_matches: Option<&ArgMatches>) -> anyhow::Result<()> {
        let arg_matches = arg_matches.unwrap();

        // TODO: implement command

        Ok(())
    }
}
