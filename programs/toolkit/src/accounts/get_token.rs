use crate::utils::arg_pubkey;
use crate::{utils::Config, ToolkitCommand};
use clap::{Arg, ArgMatches};
use solana_clap_utils::input_parsers::pubkey_of;
use solana_program::program_pack::Pack;

const ARG_TOKEN_MINT: &str = "mint";
const ARG_TOKEN_ACCOUNT: &str = "account";

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
        vec![
            arg_pubkey(ARG_TOKEN_MINT, true),
            arg_pubkey(ARG_TOKEN_ACCOUNT, false),
        ]
    }

    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>> {
        vec![]
    }

    fn handle(&self, config: &Config, arg_matches: Option<&ArgMatches>) -> anyhow::Result<()> {
        let arg_matches = arg_matches.unwrap();
        let arg_mint = pubkey_of(arg_matches, ARG_TOKEN_MINT).unwrap();
        let arg_account = pubkey_of(arg_matches, ARG_TOKEN_ACCOUNT);

        let account = config.rpc_client.get_account(&arg_mint)?;
        let mint_account = spl_token::state::Mint::unpack(&account.data).unwrap();

        println!("{:#?}", account);
        println!("{:#?}", mint_account);

        if let Some(a) = arg_account {
            let token = config.rpc_client.get_account(&a)?;
            let token_account = spl_token::state::Account::unpack(&token.data).unwrap();

            println!("{:#?}", token_account);
        }

        Ok(())
    }
}
