use clap::{Arg, ArgMatches};
use solana_clap_utils::input_parsers::{pubkey_of, value_of};
use everlend_general_pool::state::SetPoolConfigParams;
use crate::{Config, ToolkitCommand};
use crate::helpers::set_pool_config;
use crate::utils::{arg, arg_pubkey};

const ARG_POOL: &str = "pool";
const ARG_MIN_DEPOSIT: &str = "min-deposit";
const ARG_MIN_WITHDRAW: &str = "min-withdraw";

#[derive(Clone, Copy)]
pub struct SetPoolConfigCommand;

impl<'a> ToolkitCommand<'a> for SetPoolConfigCommand {
    fn get_name(&self) -> &'a str {
        return "set-pool-config";
    }

    fn get_description(&self) -> &'a str {
        return "Create or update pool config";
    }

    fn get_args(&self) -> Vec<Arg<'a, 'a>> {
        return vec![
            arg_pubkey(ARG_POOL, true).short("P").help("General pool pubkey"),
            arg(ARG_MIN_DEPOSIT, false).value_name("NUMBER").help("Minimum amount for deposit"),
            arg(ARG_MIN_WITHDRAW, false).value_name("NUMBER").help("Minimum amount for withdraw"),
        ];
    }

    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>> {
        return vec![];
    }

    fn handle(&self, config: &Config, arg_matches: Option<&ArgMatches>) -> anyhow::Result<()> {

        let arg_matches = arg_matches.unwrap();
        let pool_pubkey = pubkey_of(arg_matches, ARG_POOL).unwrap();
        let deposit_minimum = value_of::<u64>(arg_matches, ARG_MIN_DEPOSIT);
        let withdraw_minimum = value_of::<u64>(arg_matches, ARG_MIN_WITHDRAW);
        let params = SetPoolConfigParams {
            deposit_minimum,
            withdraw_minimum,
        };

        let initialized_accounts = config.get_initialized_accounts();
        set_pool_config(
            config,
            &initialized_accounts.general_pool_market,
            &pool_pubkey,
            params,
        )?;

        Ok(())
    }
}