use crate::utils::{arg_multiple, arg_pubkey, spl_create_associated_token_account};
use crate::{Config, ToolkitCommand};
use clap::{Arg, ArgMatches};
use solana_clap_utils::input_parsers::pubkey_of;
use solana_sdk::transaction::Transaction;

const ARG_MINTS: &str = "mints";
const ARG_DESTINATION_ACCOUNT: &str = "destination";

#[derive(Clone, Copy)]
pub struct FlushRewardCommand;

impl<'a> ToolkitCommand<'a> for FlushRewardCommand {
    fn get_name(&self) -> &'a str {
        "flush-reward"
    }

    fn get_description(&self) -> &'a str {
        "Flush reward from income pool"
    }

    fn get_args(&self) -> Vec<Arg<'a, 'a>> {
        vec![
            arg_pubkey(ARG_DESTINATION_ACCOUNT, true)
                .short("d")
                .help("Destination account"),
            arg_multiple(ARG_MINTS, true).short("m"),
        ]
    }

    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>> {
        vec![]
    }

    fn handle(&self, config: &Config, arg_matches: Option<&ArgMatches>) -> anyhow::Result<()> {
        let arg_matches = arg_matches.unwrap();

        let required_mints: Vec<_> = arg_matches.values_of(ARG_MINTS).unwrap().collect();
        let destination_pubkey = pubkey_of(arg_matches, ARG_DESTINATION_ACCOUNT).unwrap();

        let initialiazed_accounts = config.get_initialized_accounts();

        let mut instructions = vec![];
        for mint in required_mints {
            let token_accounts = initialiazed_accounts.token_accounts.get(mint).unwrap();

            let destination_token_account = spl_create_associated_token_account(
                config,
                &destination_pubkey,
                &token_accounts.mint,
            )?;

            instructions.push(everlend_income_pools::instruction::flush_reward(
                &everlend_income_pools::id(),
                &initialiazed_accounts.income_pool_market,
                &token_accounts.mint,
                &destination_token_account,
                &config.fee_payer.pubkey(),
            ));
        }

        let tx = Transaction::new_with_payer(&instructions, Some(&config.fee_payer.pubkey()));

        config.sign_and_send_and_confirm_transaction(tx, vec![config.fee_payer.as_ref()])?;

        Ok(())
    }
}
