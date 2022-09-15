use crate::liquidity_mining::quarry_raw_test;
use crate::{
    utils::{arg, Config},
    ToolkitCommand,
};
use clap::{Arg, ArgMatches};
use solana_clap_utils::input_parsers::value_of;
use std::{thread, time};

const ARG_TOKEN: &str = "token";

#[derive(Clone, Copy)]
pub struct TestQuarryMiningRawCommand;

impl<'a> ToolkitCommand<'a> for TestQuarryMiningRawCommand {
    fn get_name(&self) -> &'a str {
        "test-quarry-mining-raw"
    }

    fn get_description(&self) -> &'a str {
        "Test Quarry mining raw"
    }

    fn get_args(&self) -> Vec<Arg<'a, 'a>> {
        vec![arg(ARG_TOKEN, true)]
    }

    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>> {
        vec![]
    }

    fn handle(&self, config: &Config, arg_matches: Option<&ArgMatches>) -> anyhow::Result<()> {
        let arg_matches = arg_matches.unwrap();
        let token = value_of::<String>(arg_matches, ARG_TOKEN).unwrap();

        let amount = 100_000;
        println!("depositing {}", amount);
        quarry_raw_test::stake_tokens(config, &token, amount)?;
        println!("stake tokens finished");
        thread::sleep(time::Duration::from_secs(15));
        quarry_raw_test::claim_mining_rewards(config, &token)?;
        println!("claim rewards finished");
        quarry_raw_test::withdraw_tokens(config, &token, amount - 1000)?;
        println!("withdraw tokens finished");

        Ok(())
    }
}
