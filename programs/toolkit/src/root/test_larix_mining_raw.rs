use crate::{utils::Config, ToolkitCommand};
use clap::{Arg, ArgMatches};

#[derive(Clone, Copy)]
pub struct TestLarixMiningRawCommand;

impl<'a> ToolkitCommand<'a> for TestLarixMiningRawCommand {
    fn get_name(&self) -> &'a str {
        return "test-larix-mining-raw";
    }

    fn get_description(&self) -> &'a str {
        return "";
    }

    fn get_args(&self) -> Vec<Arg<'a, 'a>> {
        return vec![];
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
