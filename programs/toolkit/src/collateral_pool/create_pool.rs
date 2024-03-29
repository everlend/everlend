use crate::accounts_config::CollateralPoolAccounts;
use crate::helpers::{create_collateral_pool, create_transit};
use crate::utils::{arg, arg_multiple, get_asset_maps};
use crate::{Config, ToolkitCommand};
use clap::{Arg, ArgMatches};
use solana_clap_utils::input_parsers::value_of;
use solana_program::pubkey::Pubkey;

const ARG_MONEY_MARKET: &str = "money-market";
const ARG_MINTS: &str = "mints";

pub struct CreatePoolCommand;

impl<'a> ToolkitCommand<'a> for CreatePoolCommand {
    fn get_name(&self) -> &'a str {
        "create-pool"
    }

    fn get_description(&self) -> &'a str {
        "Create a new collateral pool"
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
        let mut initialiazed_accounts = config.get_initialized_accounts();

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

            if collateral_mint.eq(&Pubkey::default()) {
                println!("collateral_mint for {} missed", key);
                continue;
            }

            let pool_pubkeys =
                create_collateral_pool(config, &collateral_pool_market_pubkey, &collateral_mint)?;

            create_transit(
                config,
                &initialiazed_accounts.depositor,
                &collateral_mint,
                None,
            )?;

            let money_market_accounts = CollateralPoolAccounts {
                pool: pool_pubkeys.pool,
                pool_token_account: pool_pubkeys.token_account,
                token_mint: collateral_mint,
            };

            initialiazed_accounts
                .token_accounts
                .get_mut(key)
                .unwrap()
                .collateral_pools[money_market_index] = money_market_accounts;
        }

        initialiazed_accounts
            .save(config.accounts_path.as_str())
            .unwrap();

        Ok(())
    }
}
