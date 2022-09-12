use clap::{Arg, ArgMatches};
use larix_lending::state::reserve::Reserve;
use solana_program::program_pack::Pack;
use solana_program_test::{find_file, read_file};
use crate::{Config, ToolkitCommand};
use crate::utils::download_account;

const LARIX_RESERVE_PATH: &str = "../tests/tests/fixtures/larix/reserve_sol.bin";

#[derive(Clone, Copy)]
pub struct SaveLarixAccountsCommand;

impl<'a> ToolkitCommand<'a> for SaveLarixAccountsCommand {
    fn get_name(&self) -> &'a str {
        return "save-larix-accounts";
    }

    fn get_description(&self) -> &'a str {
        return "Save Larix accounts";
    }

    fn get_args(&self) -> Vec<Arg<'a, 'a>> {
        return vec![];
    }

    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>> {
        return vec![];
    }

    fn handle(&self, _config: &Config, _arg_matches: Option<&ArgMatches>) -> anyhow::Result<()> {
        let mut reserve_data = read_file(find_file(LARIX_RESERVE_PATH).unwrap());
        let reserve = Reserve::unpack_from_slice(reserve_data.as_mut_slice()).unwrap();

        download_account(
            &reserve.liquidity.supply_pubkey,
            "larix",
            "liquidity_supply",
        );
        download_account(
            &reserve.liquidity.fee_receiver,
            "larix",
            "liquidity_fee_receiver",
        );
        download_account(&reserve.collateral.mint_pubkey, "larix", "collateral_mint");
        download_account(
            &reserve.collateral.supply_pubkey,
            "larix",
            "collateral_supply",
        );

        Ok(())
    }
}