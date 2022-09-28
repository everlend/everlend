use crate::helpers::set_pool_config;
use crate::utils::{arg, arg_pubkey};
use crate::{Config, ToolkitCommand};
use clap::{Arg, ArgMatches};
use everlend_general_pool::find_pool_program_address;
use everlend_general_pool::state::SetPoolConfigParams;
use solana_clap_utils::input_parsers::{pubkey_of, value_of};
use solana_program::program_pack::Pack;

const ARG_MINT: &str = "mint";
const ARG_MIN_DEPOSIT: &str = "min-deposit";
const ARG_MIN_WITHDRAW: &str = "min-withdraw";

#[derive(Clone, Copy)]
pub struct SetPoolConfigCommand;

impl<'a> ToolkitCommand<'a> for SetPoolConfigCommand {
    fn get_name(&self) -> &'a str {
        "set-pool-config"
    }

    fn get_description(&self) -> &'a str {
        "Create or update pool config"
    }

    fn get_args(&self) -> Vec<Arg<'a, 'a>> {
        vec![
            arg_pubkey(ARG_MINT, true),
            arg(ARG_MIN_DEPOSIT, false)
                .value_name("DECIMAL")
                .help("Minimum amount for deposit (e.g. 0.01 or 1)"),
            arg(ARG_MIN_WITHDRAW, false)
                .value_name("DECIMAL")
                .help("Minimum amount for withdraw (e.g. 0.01 or 1)"),
        ]
    }

    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>> {
        vec![]
    }

    fn handle(&self, config: &Config, arg_matches: Option<&ArgMatches>) -> anyhow::Result<()> {
        let arg_matches = arg_matches.unwrap();
        let initialized_accounts = config.get_initialized_accounts();

        let arg_mint = pubkey_of(arg_matches, ARG_MINT).unwrap();
        let arg_deposit_minimum: Option<f64> = value_of(arg_matches, ARG_MIN_DEPOSIT);
        let arg_withdraw_minimum: Option<f64> = value_of(arg_matches, ARG_MIN_WITHDRAW);

        let account = config.rpc_client.get_account(&arg_mint)?;
        let mint_account = spl_token::state::Mint::unpack(&account.data).unwrap();

        let (pool, _) = find_pool_program_address(
            &everlend_general_pool::id(),
            &initialized_accounts.general_pool_market,
            &arg_mint,
        );

        let mut params = SetPoolConfigParams {
            deposit_minimum: None,
            withdraw_minimum: None,
        };

        if let Some(min) = arg_deposit_minimum {
            params.deposit_minimum =
                Some((min * (10_u32.pow(mint_account.decimals as u32) as f64)) as u64);
        }
        if let Some(min) = arg_withdraw_minimum {
            params.withdraw_minimum =
                Some((min * (10_u32.pow(mint_account.decimals as u32) as f64)) as u64);
        }

        println!(
            "Pool: {} deposit-min: {}, withdraw-min: {}",
            pool,
            params.deposit_minimum.unwrap_or_default(),
            params.withdraw_minimum.unwrap_or_default()
        );

        set_pool_config(
            config,
            &initialized_accounts.general_pool_market,
            &pool,
            params,
        )?;

        Ok(())
    }
}
