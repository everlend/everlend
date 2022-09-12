use clap::{Arg, ArgMatches};
use solana_clap_utils::input_parsers::keypair_of;
use crate::{arg_keypair, Config, ToolkitCommand};
use crate::helpers::init_liquidity_oracle;

const ARG_KEYPAIR: &str = "keypair";

#[derive(Clone, Copy)]
pub struct CreateLiquidityOracleCommand;

impl<'a> ToolkitCommand<'a> for CreateLiquidityOracleCommand {
    fn get_name(&self) -> &'a str {
        return "create-liquidity-oracle";
    }

    fn get_description(&self) -> &'a str {
        return "Create a new liquidity oracle";
    }

    fn get_args(&self) -> Vec<Arg<'a, 'a>> {
        return vec![
            arg_keypair(ARG_KEYPAIR, false),
        ]
    }

    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>> {
        return vec![];
    }

    fn handle(&self, config: &Config, arg_matches: Option<&ArgMatches>) -> anyhow::Result<()> {
        let arg_matches = arg_matches.unwrap();
        let keypair = keypair_of(arg_matches, ARG_KEYPAIR);

        let mut initialiazed_accounts = config.get_initialized_accounts();

        let liquidity_oracle_pubkey = init_liquidity_oracle(config, keypair)?;

        initialiazed_accounts.liquidity_oracle = liquidity_oracle_pubkey;

        initialiazed_accounts
            .save(&format!("accounts.{}.yaml", config.network))
            .unwrap();

        Ok(())
    }
}