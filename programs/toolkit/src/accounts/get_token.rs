use crate::utils::arg_pubkey;
use crate::{utils::Config, ToolkitCommand};
use clap::{Arg, ArgMatches};
use solana_clap_utils::input_parsers::pubkey_of;

const ARG_TOKEN_MINT: &str = "mint";

#[derive(Clone, Copy)]
pub struct GetTokenCommand;

impl<'a> ToolkitCommand<'a> for GetTokenCommand {
    fn get_name(&self) -> &'a str {
        "get-token"
    }

    fn get_description(&self) -> &'a str {
        "Get token"
    }

    fn get_args(&self) -> Vec<Arg<'a, 'a>> {
        vec![arg_pubkey(ARG_TOKEN_MINT, true)]
    }

    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>> {
        vec![]
    }

    fn handle(&self, config: &Config, arg_matches: Option<&ArgMatches>) -> anyhow::Result<()> {
        let arg_matches = arg_matches.unwrap();
        let mint = pubkey_of(arg_matches, ARG_TOKEN_MINT).unwrap();

        let mint_account: spl_token::state::Mint = config.get_account_unpack(&mint)?;
        println!("{:#?}", mint_account);

        Ok(())
    }
}
