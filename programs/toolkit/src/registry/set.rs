use crate::helpers::{update_registry, update_registry_markets};
use crate::utils::{arg_pubkey, REFRESH_INCOME_INTERVAL};
use crate::{utils::Config, ToolkitCommand};
use clap::{Arg, ArgMatches};
use everlend_registry::instructions::{UpdateRegistryData, UpdateRegistryMarketsData};
use everlend_registry::state::DistributionPubkeys;
use everlend_utils::integrations::MoneyMarket;
use everlend_utils::integrations::MoneyMarket::Frakt;
use solana_clap_utils::input_parsers::pubkey_of;
use std::convert::TryInto;

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

        let mut money_markets: Vec<MoneyMarket> = vec![];
        money_markets[0] = MoneyMarket::PortFinance {
            money_market_program_id: default_accounts.port_finance.program_id,
        };
        money_markets[1] = MoneyMarket::Larix {
            money_market_program_id: default_accounts.larix.program_id,
        };
        money_markets[2] = MoneyMarket::Solend {
            money_market_program_id: default_accounts.solend.program_id,
            lending_market: default_accounts.solend.lending_market,
        };
        money_markets[3] = MoneyMarket::Tulip {
            money_market_program_id: default_accounts.tulip.program_id,
        };
        money_markets[4] = MoneyMarket::Francium {
            money_market_program_id: default_accounts.francium.program_id,
        };
        money_markets[5] = MoneyMarket::Jet {
            money_market_program_id: default_accounts.jet.program_id,
        };
        for liquidity_pool in default_accounts.frakt.liquidity_pools {
            money_markets.push(Frakt {
                money_market_program_id: default_accounts.frakt.program_id,
                liquidity_pool,
            });
        }

        for _ in money_markets.len()..10 {
            money_markets.push(Default::default());
        }

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
                money_markets: Some(money_markets.try_into().unwrap()),
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
