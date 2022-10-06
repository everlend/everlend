use crate::utils::get_asset_maps;
use crate::{Config, ToolkitCommand};
use clap::{Arg, ArgMatches};

pub struct InfoReserveLiquidityCommand;

impl<'a> ToolkitCommand<'a> for InfoReserveLiquidityCommand {
    fn get_name(&self) -> &'a str {
        "info-reserve-liquidity"
    }

    fn get_description(&self) -> &'a str {
        "Info reserve accounts"
    }

    fn get_args(&self) -> Vec<Arg<'a, 'a>> {
        vec![]
    }

    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>> {
        vec![]
    }

    fn handle(&self, config: &Config, _arg_matches: Option<&ArgMatches>) -> anyhow::Result<()> {
        let default_accounts = config.get_default_accounts();
        let initialiazed_accounts = config.get_initialized_accounts();

        let (mint_map, _) = get_asset_maps(default_accounts);

        for (token, mint) in mint_map.iter() {
            let (liquidity_reserve_transit_pubkey, _) =
                everlend_depositor::find_transit_program_address(
                    &everlend_depositor::id(),
                    &initialiazed_accounts.depositor,
                    mint,
                    "reserve",
                );

            let liquidity_reserve_transit = config
                .get_account_unpack::<spl_token::state::Account>(
                    &liquidity_reserve_transit_pubkey,
                )?;

            println!(
                "{} - {:?}: {:?}",
                token, liquidity_reserve_transit_pubkey, liquidity_reserve_transit.amount
            );
        }

        Ok(())
    }
}
