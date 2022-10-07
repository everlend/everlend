use crate::helpers::{init_registry, update_registry, update_registry_markets};
use crate::utils::REFRESH_INCOME_INTERVAL;
use crate::{
    utils::{arg_keypair, Config},
    ToolkitCommand,
};
use clap::{Arg, ArgMatches};
use everlend_registry::instructions::{UpdateRegistryData, UpdateRegistryMarketsData};
use everlend_registry::state::DistributionPubkeys;
use solana_clap_utils::input_parsers::keypair_of;

const ARG_REGISTRY: &str = "registry";

pub struct InitRegistryCommand;

impl<'a> ToolkitCommand<'a> for InitRegistryCommand {
    fn get_name(&self) -> &'a str {
        "init"
    }

    fn get_description(&self) -> &'a str {
        "init registry"
    }

    fn get_args(&self) -> Vec<Arg<'a, 'a>> {
        vec![arg_keypair(ARG_REGISTRY, true)]
    }

    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>> {
        vec![]
    }

    fn handle(&self, config: &Config, arg_matches: Option<&ArgMatches>) -> anyhow::Result<()> {
        let arg_matches = arg_matches.unwrap();
        let keypair = keypair_of(arg_matches, ARG_REGISTRY);
        let payer_pubkey = config.fee_payer.pubkey();
        println!("Fee payer: {}", payer_pubkey);

        let registry_pubkey = init_registry(config, keypair)?;

        let default_accounts = config.get_default_accounts();
        let initialized_accounts = config.get_initialized_accounts();

        let mut money_market_program_ids = DistributionPubkeys::default();
        money_market_program_ids[0] = default_accounts.port_finance.program_id;
        money_market_program_ids[1] = default_accounts.larix.program_id;
        money_market_program_ids[2] = default_accounts.solend.program_id;
        money_market_program_ids[3] = default_accounts.tulip.program_id;
        money_market_program_ids[4] = default_accounts.francium.program_id;
        money_market_program_ids[5] = default_accounts.mango.program_id;

        let mut collateral_pool_markets = DistributionPubkeys::default();
        let initialized_collateral_pool_markets = &initialized_accounts.collateral_pool_markets;
        collateral_pool_markets[..initialized_collateral_pool_markets.len()]
            .copy_from_slice(initialized_collateral_pool_markets);

        update_registry(
            config,
            &registry_pubkey,
            UpdateRegistryData {
                general_pool_market: Some(initialized_accounts.general_pool_market),
                income_pool_market: Some(initialized_accounts.income_pool_market),
                liquidity_oracle: Some(initialized_accounts.liquidity_oracle),
                refresh_income_interval: Some(REFRESH_INCOME_INTERVAL),
            },
        )?;

        update_registry_markets(
            config,
            &registry_pubkey,
            UpdateRegistryMarketsData {
                money_markets: Some(money_market_program_ids),
                collateral_pool_markets: Some(collateral_pool_markets),
            },
        )?;

        let mut initialized_accounts = config.get_initialized_accounts();
        initialized_accounts.payer = payer_pubkey;
        initialized_accounts.registry = registry_pubkey;

        initialized_accounts
            .save(config.accounts_path.as_str())
            .unwrap();

        Ok(())
    }
}
