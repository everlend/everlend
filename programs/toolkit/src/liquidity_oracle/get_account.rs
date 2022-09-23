use crate::utils::arg_pubkey;
use crate::{Config, ToolkitCommand};
use clap::{Arg, ArgMatches};
use everlend_liquidity_oracle::find_token_oracle_program_address;
use everlend_liquidity_oracle::state::TokenOracle;
use solana_clap_utils::input_parsers::pubkey_of;

const ARG_MINT: &str = "mint";

#[derive(Clone, Copy)]
pub struct GetTokenOracleAccountCommand;

impl<'a> ToolkitCommand<'a> for GetTokenOracleAccountCommand {
    fn get_name(&self) -> &'a str {
        "get-token-oracle-account"
    }

    fn get_description(&self) -> &'a str {
        "Get pool oracle account"
    }

    fn get_args(&self) -> Vec<Arg<'a, 'a>> {
        vec![arg_pubkey(ARG_MINT, true).help("Token mint")]
    }

    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>> {
        vec![]
    }

    fn handle(&self, config: &Config, arg_matches: Option<&ArgMatches>) -> anyhow::Result<()> {
        let arg_matches = arg_matches.unwrap();
        let mint = pubkey_of(arg_matches, ARG_MINT).unwrap();
        let acc = config.get_initialized_accounts();

        let (token_oracle_pubkey, _) = find_token_oracle_program_address(
            &everlend_liquidity_oracle::id(),
            &acc.liquidity_oracle,
            &mint,
        );

        let oracle: TokenOracle = config.get_account_unpack(&token_oracle_pubkey)?;
        println!("{:#?}", oracle);

        Ok(())
    }
}
