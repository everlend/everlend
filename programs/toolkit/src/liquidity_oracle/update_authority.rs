use crate::helpers::update_oracle_authority;
use crate::{arg_keypair, Config, ToolkitCommand};
use clap::{Arg, ArgMatches};
use solana_clap_utils::input_parsers::keypair_of;
use solana_sdk::signer::Signer;

const ARG_AUTHORITY: &str = "authority";
const ARG_NEW_AUTHORITY: &str = "new-authority";

#[derive(Clone, Copy)]
pub struct UpdateAuthorityCommand;

impl<'a> ToolkitCommand<'a> for UpdateAuthorityCommand {
    fn get_name(&self) -> &'a str {
        "update-liquidity-oracle-authority"
    }

    fn get_description(&self) -> &'a str {
        "Update liquidity oracle authority"
    }

    fn get_args(&self) -> Vec<Arg<'a, 'a>> {
        vec![
            arg_keypair(ARG_AUTHORITY, true)
                .value_name("AUTHORITY")
                .help("Old manager keypair"),
            arg_keypair(ARG_NEW_AUTHORITY, true)
                .value_name("NEW-AUTHORITY")
                .help("New manager keypair"),
        ]
    }

    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>> {
        vec![]
    }

    fn handle(&self, config: &Config, arg_matches: Option<&ArgMatches>) -> anyhow::Result<()> {
        let arg_matches = arg_matches.unwrap();
        let authority = keypair_of(arg_matches, ARG_AUTHORITY).unwrap();
        let new_authority = keypair_of(arg_matches, ARG_NEW_AUTHORITY).unwrap();

        let initialiazed_accounts = config.get_initialized_accounts();

        println!(
            "oracle {} new authority {}",
            initialiazed_accounts.liquidity_oracle,
            new_authority.pubkey()
        );

        update_oracle_authority(
            config,
            initialiazed_accounts.liquidity_oracle,
            authority,
            new_authority,
        )?;

        Ok(())
    }
}
