use crate::helpers::{update_registry, update_registry_markets};
use crate::utils::{arg_pubkey, REFRESH_INCOME_INTERVAL};
use crate::{
    utils::{Config},
    ToolkitCommand,
};
use clap::{Arg, ArgMatches};
use everlend_registry::instructions::{UpdateRegistryData, UpdateRegistryMarketsData};
use everlend_registry::state::{DistributionPubkeys, MoneyMarket, MoneyMarkets};
use solana_clap_utils::input_parsers::pubkey_of;

const ARG_REGISTRY: &str = "registry";

pub struct SetRegistryCommand;

impl<'a> ToolkitCommand<'a> for SetRegistryCommand {
    fn get_name(&self) -> &'a str {
        "set"
    }

    fn get_description(&self) -> &'a str {
        "set registry"
    }

    fn get_args(&self) -> Vec<Arg<'a, 'a>> {
        vec![arg_pubkey(ARG_REGISTRY, true)]
    }

    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>> {
        vec![]
    }

    fn handle(&self, config: &Config, arg_matches: Option<&ArgMatches>) -> anyhow::Result<()> {
        let arg_matches = arg_matches.unwrap();
        let registry_pubkey = pubkey_of(arg_matches, ARG_REGISTRY).unwrap();
        let payer_pubkey = config.fee_payer.pubkey();
        println!("Fee payer: {}", payer_pubkey);

        let default_accounts = config.get_default_accounts();
        let initialized_accounts = config.get_initialized_accounts();

        let mut money_market_program_ids = MoneyMarkets::default();
        money_market_program_ids[0] = MoneyMarket {
            id: everlend_utils::integrations::MoneyMarket::PortFinance,
            program_id: default_accounts.port_finance[0].program_id,
            lending_market: default_accounts.port_finance[0].lending_market,
        };
        money_market_program_ids[1] = MoneyMarket {
            id: everlend_utils::integrations::MoneyMarket::Larix,
            program_id: default_accounts.larix[0].program_id,
            lending_market: default_accounts.larix[0].lending_market,
        };
        money_market_program_ids[2] = MoneyMarket {
            id: everlend_utils::integrations::MoneyMarket::Solend,
            program_id: default_accounts.solend[0].program_id,
            lending_market: default_accounts.solend[0].lending_market,
        };
        money_market_program_ids[3] = MoneyMarket {
            id: everlend_utils::integrations::MoneyMarket::Tulip,
            program_id: default_accounts.tulip[0].program_id,
            lending_market: default_accounts.tulip[0].lending_market,
        };
        money_market_program_ids[4] = MoneyMarket {
            id: everlend_utils::integrations::MoneyMarket::Francium,
            program_id: default_accounts.francium[0].program_id,
            lending_market: default_accounts.francium[0].lending_market,
        };
        money_market_program_ids[5] = MoneyMarket {
            id: everlend_utils::integrations::MoneyMarket::Jet,
            program_id: default_accounts.jet[0].program_id,
            lending_market: Default::default(),
        };

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
