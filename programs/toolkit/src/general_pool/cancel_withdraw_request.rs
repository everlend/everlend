use clap::{Arg, ArgMatches};
use solana_clap_utils::input_parsers::pubkey_of;
use crate::{Config, ToolkitCommand};
use crate::helpers::cancel_withdraw_request;
use crate::utils::arg_pubkey;

const ARG_REQUEST: &str = "request";

#[derive(Clone, Copy)]
pub struct CancelWithdrawRequestCommand;

impl<'a> ToolkitCommand<'a> for CancelWithdrawRequestCommand {
    fn get_name(&self) -> &'a str {
        return "cancel-withdraw-request";
    }

    fn get_description(&self) -> &'a str {
        return "Cancel withdraw request";
    }

    fn get_args(&self) -> Vec<Arg<'a, 'a>> {
        return vec![
            arg_pubkey(ARG_REQUEST, true).help("Withdrawal request pubkey"),
        ];
    }

    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>> {
        return vec![];
    }

    fn handle(&self, config: &Config, arg_matches: Option<&ArgMatches>) -> anyhow::Result<()> {
        let arg_matches = arg_matches.unwrap();
        let request_pubkey = pubkey_of(arg_matches, ARG_REQUEST).unwrap();
        let initialiazed_accounts = config.get_initialized_accounts();

        let withdrawal_request = config
            .get_account_unpack::<everlend_general_pool::state::WithdrawalRequest>(
                &request_pubkey,
            )?;

        let general_pool = config
            .get_account_unpack::<everlend_general_pool::state::Pool>(&withdrawal_request.pool)?;

        cancel_withdraw_request(
            config,
            &initialiazed_accounts.general_pool_market,
            &withdrawal_request.pool,
            &withdrawal_request.source,
            &general_pool.token_mint,
            &general_pool.pool_mint,
            &withdrawal_request.from,
        )?;

        Ok(())
    }
}