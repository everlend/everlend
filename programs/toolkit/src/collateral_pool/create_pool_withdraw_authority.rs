use crate::helpers::create_pool_withdraw_authority;
use crate::utils::{arg, arg_multiple, get_asset_maps};
use crate::{Config, ToolkitCommand};
use clap::{Arg, ArgMatches};
use everlend_collateral_pool::find_pool_program_address;
use everlend_utils::find_program_address;
use solana_clap_utils::input_parsers::value_of;
use solana_program::pubkey::Pubkey;

const ARG_MONEY_MARKET: &str = "money-market";
const ARG_MINTS: &str = "mints";
#[derive(Clone, Copy)]
pub struct CreatePoolWithdrawAuthorityCommand;

impl<'a> ToolkitCommand<'a> for CreatePoolWithdrawAuthorityCommand {
    fn get_name(&self) -> &'a str {
        "create-pool-withdraw-authority"
    }

    fn get_description(&self) -> &'a str {
        "Create pool withdraw authority"
    }

    fn get_args(&self) -> Vec<Arg<'a, 'a>> {
        vec![
            arg(ARG_MONEY_MARKET, true)
                .value_name("NUMBER")
                .help("Money market index"),
            arg_multiple(ARG_MINTS, true).short("m"),
        ]
    }

    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>> {
        vec![]
    }

    fn handle(&self, config: &Config, arg_matches: Option<&ArgMatches>) -> anyhow::Result<()> {
        let arg_matches = arg_matches.unwrap();
        let money_market = value_of::<usize>(arg_matches, ARG_MONEY_MARKET).unwrap();
        let required_mints: Vec<_> = arg_matches.values_of(ARG_MINTS).unwrap().collect();

        let default_accounts = config.get_default_accounts();
        let initialiazed_accounts = config.get_initialized_accounts();

        let (_, collateral_mint_map) = get_asset_maps(default_accounts);
        let money_market_index = money_market as usize;
        let collateral_pool_market_pubkey =
            initialiazed_accounts.collateral_pool_markets[money_market_index];
        if collateral_pool_market_pubkey.eq(&Pubkey::default()) {
            println!("collateral_pool_market_pubkey is empty. Create it first");
            return Ok(());
        }

        for key in required_mints {
            let collateral_mint =
                collateral_mint_map.get(key).unwrap()[money_market_index].unwrap();
            let (pool_pubkey, _) = find_pool_program_address(
                &everlend_collateral_pool::id(),
                &collateral_pool_market_pubkey,
                &collateral_mint,
            );

            let (depositor_authority, _) =
                find_program_address(&everlend_depositor::id(), &initialiazed_accounts.depositor);

            create_pool_withdraw_authority(
                config,
                &collateral_pool_market_pubkey,
                &pool_pubkey,
                &depositor_authority,
                &config.fee_payer.pubkey(),
            )?;
        }

        Ok(())
    }
}
