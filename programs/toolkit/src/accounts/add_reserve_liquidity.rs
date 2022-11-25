use crate::utils::{arg, arg_amount, get_asset_maps, spl_token_transfer};
use crate::{Config, ToolkitCommand};
use clap::{Arg, ArgMatches};
use everlend_utils::PDA;
use solana_clap_utils::input_parsers::value_of;
use spl_associated_token_account::get_associated_token_address;

const ARG_MINT: &str = "mint";
const ARG_AMOUNT: &str = "amount";

#[derive(Clone, Copy)]
pub struct AddReserveLiquidityCommand;

impl<'a> ToolkitCommand<'a> for AddReserveLiquidityCommand {
    fn get_name(&self) -> &'a str {
        "add-reserve-liquidity"
    }

    fn get_description(&self) -> &'a str {
        "Transfer liquidity to reserve account"
    }

    fn get_args(&self) -> Vec<Arg<'a, 'a>> {
        vec![
            arg(ARG_MINT, true).short("m"),
            arg_amount(ARG_AMOUNT, true).help("Liquidity amount"),
        ]
    }

    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>> {
        vec![]
    }

    fn handle(&self, config: &Config, arg_matches: Option<&ArgMatches>) -> anyhow::Result<()> {
        let arg_matches = arg_matches.unwrap();

        let mint_key = arg_matches.value_of(ARG_MINT).unwrap();
        let amount = value_of::<u64>(arg_matches, ARG_AMOUNT).unwrap();

        let payer_pubkey = config.fee_payer.pubkey();
        let default_accounts = config.get_default_accounts();
        let initialiazed_accounts = config.get_initialized_accounts();

        let (mint_map, _) = get_asset_maps(default_accounts);
        let mint = mint_map.get(mint_key).unwrap();

        let (liquidity_reserve_transit_pubkey, _) = everlend_depositor::TransitPDA {
            depositor: initialiazed_accounts.depositor.clone(),
            mint: mint.clone(),
            seed: "reserve",
        }
        .find_address(&everlend_depositor::id());

        println!(
            "liquidity_reserve_transit_pubkey = {:?}",
            liquidity_reserve_transit_pubkey
        );

        let token_account = get_associated_token_address(&payer_pubkey, mint);

        println!(
            "Transfer {} lamports of {} to reserve liquidity account",
            amount, mint_key
        );

        spl_token_transfer(
            config,
            &token_account,
            &liquidity_reserve_transit_pubkey,
            amount,
        )?;

        Ok(())
    }
}
