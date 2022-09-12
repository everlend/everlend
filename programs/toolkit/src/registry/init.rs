use crate::{
    utils::{arg_keypair, Config},
    ToolkitCommand,
};
use clap::{Arg, ArgMatches};
use solana_clap_utils::input_parsers::keypair_of;
use solana_program::pubkey::Pubkey;
use everlend_registry::state::{RegistryPrograms, RegistryRootAccounts, RegistrySettings, TOTAL_DISTRIBUTIONS};
use crate::helpers::{init_registry, set_registry_config};
use crate::utils::REFRESH_INCOME_INTERVAL;

const ARG_REGISTRY: &str = "registry";

pub struct InitRegistryCommand;

impl<'a> ToolkitCommand<'a> for InitRegistryCommand {
    fn get_name(&self) -> &'a str {
        return "init";
    }

    fn get_description(&self) -> &'a str {
        return "init registry";
    }

    fn get_args(&self) -> Vec<Arg<'a, 'a>> {
        return vec![arg_keypair(ARG_REGISTRY, true)];
    }

    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>> {
        return vec![];
    }

    fn handle(&self, config: &Config, arg_matches: Option<&ArgMatches>) -> anyhow::Result<()> {
        let arg_matches = arg_matches.unwrap();

        let keypair = keypair_of(arg_matches, ARG_REGISTRY);

        let payer_pubkey = config.fee_payer.pubkey();
        println!("Fee payer: {}", payer_pubkey);

        let default_accounts = config.get_default_accounts();
        let mut initialized_accounts = config.get_initialized_accounts();

        let mm_pool_markets = &initialized_accounts.mm_pool_markets;

        let registry_pubkey = init_registry(config, keypair)?;
        let mut programs = RegistryPrograms {
            general_pool_program_id: everlend_general_pool::id(),
            collateral_pool_program_id: everlend_collateral_pool::id(),
            liquidity_oracle_program_id: everlend_liquidity_oracle::id(),
            depositor_program_id: everlend_depositor::id(),
            income_pools_program_id: everlend_income_pools::id(),
            money_market_program_ids: [Pubkey::default(); TOTAL_DISTRIBUTIONS],
        };
        programs.money_market_program_ids[0] = default_accounts.port_finance.program_id;
        programs.money_market_program_ids[1] = default_accounts.larix.program_id;
        programs.money_market_program_ids[2] = default_accounts.solend.program_id;
        programs.money_market_program_ids[3] = default_accounts.tulip.program_id;

        println!("programs = {:#?}", programs);

        let mut collateral_pool_markets: [Pubkey; TOTAL_DISTRIBUTIONS] = Default::default();
        collateral_pool_markets[..mm_pool_markets.len()].copy_from_slice(mm_pool_markets);

        let roots = RegistryRootAccounts {
            general_pool_market: initialized_accounts.general_pool_market,
            income_pool_market: initialized_accounts.income_pool_market,
            collateral_pool_markets,
            liquidity_oracle: initialized_accounts.liquidity_oracle,
        };

        println!("roots = {:#?}", &roots);

        set_registry_config(
            config,
            &registry_pubkey,
            programs,
            roots,
            RegistrySettings {
                refresh_income_interval: REFRESH_INCOME_INTERVAL,
            },
        )?;

        initialized_accounts.payer = payer_pubkey;
        initialized_accounts.registry = registry_pubkey;

        initialized_accounts
            .save(&format!("accounts.{}.yaml", config.network))
            .unwrap();

        Ok(())
    }
}
