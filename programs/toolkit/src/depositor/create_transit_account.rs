use crate::helpers::create_transit;
use crate::utils::{arg, arg_pubkey};
use crate::{Config, ToolkitCommand};
use clap::{Arg, ArgMatches};
use solana_clap_utils::input_parsers::{pubkey_of, value_of};

const ARG_SEED: &str = "seed";
const ARG_TOKEN_MINT: &str = "token-mint";

#[derive(Clone, Copy)]
pub struct CreateDepositorTransitAccountCommand;

impl<'a> ToolkitCommand<'a> for CreateDepositorTransitAccountCommand {
    fn get_name(&self) -> &'a str {
        return "create-transit-token-account";
    }

    fn get_description(&self) -> &'a str {
        return "Run create depositor transit token account";
    }

    fn get_args(&self) -> Vec<Arg<'a, 'a>> {
        return vec![
            arg(ARG_SEED, true).value_name("SEED").help("Transit seed"),
            arg_pubkey(ARG_TOKEN_MINT, true).help("Rewards token mint"),
        ];
    }

    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>> {
        return vec![];
    }

    fn handle(&self, config: &Config, arg_matches: Option<&ArgMatches>) -> anyhow::Result<()> {
        let arg_matches = arg_matches.unwrap();
        let token_mint = pubkey_of(arg_matches, ARG_TOKEN_MINT).unwrap();
        let seed = value_of::<String>(arg_matches, ARG_SEED);

        let initialized_accounts = config.get_initialized_accounts();

        println!("Token mint {}. Seed {:?}", token_mint, seed);
        create_transit(config, &initialized_accounts.depositor, &token_mint, seed)?;

        Ok(())
    }
}
