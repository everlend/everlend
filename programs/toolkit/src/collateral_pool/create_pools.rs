use crate::accounts_config::CollateralPoolAccounts;
use crate::helpers::{
    create_collateral_market, create_collateral_pool, create_transit, PoolPubkeys,
};
use crate::utils::get_asset_maps;
use crate::{Config, ToolkitCommand};
use clap::{Arg, ArgMatches};
use everlend_registry::state::TOTAL_DISTRIBUTIONS;
use solana_client::client_error::ClientError;
use solana_program::pubkey::Pubkey;
use std::str::FromStr;

#[derive(Clone, Copy)]
pub struct CreatePoolsCommand;

impl<'a> ToolkitCommand<'a> for CreatePoolsCommand {
    fn get_name(&self) -> &'a str {
        "create-pools"
    }

    fn get_description(&self) -> &'a str {
        "Create collateral pools"
    }

    fn get_args(&self) -> Vec<Arg<'a, 'a>> {
        vec![]
    }

    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>> {
        vec![]
    }

    fn handle(&self, config: &Config, _arg_matches: Option<&ArgMatches>) -> anyhow::Result<()> {
        let collateral_pool_markets = vec![
            create_collateral_market(config, None)?,
            create_collateral_market(config, None)?,
            create_collateral_market(config, None)?,
        ];
        let mut initialized_accounts = config.get_initialized_accounts();
        initialized_accounts.collateral_pool_markets = collateral_pool_markets;

        let default_accounts = config.get_default_accounts();

        let (_, collateral_mint_map) = get_asset_maps(default_accounts);

        let mut collateral_pool_markets: [Pubkey; TOTAL_DISTRIBUTIONS] = Default::default();
        collateral_pool_markets[..initialized_accounts.collateral_pool_markets.len()]
            .copy_from_slice(&initialized_accounts.collateral_pool_markets);

        let token_accounts = initialized_accounts.token_accounts.iter_mut();
        let depositor_pubkey = &initialized_accounts.depositor;
        for pair in token_accounts {
            let collateral_mints: Vec<(Pubkey, Pubkey)> = collateral_mint_map
                .get(pair.0)
                .unwrap()
                .iter()
                .zip(initialized_accounts.collateral_pool_markets.iter())
                .filter_map(|(collateral_mint, mm_pool_market_pubkey)| {
                    collateral_mint.map(|coll_mint| (coll_mint, *mm_pool_market_pubkey))
                })
                .collect();

            let mm_pool_collection = collateral_mints
                .iter()
                .map(|(collateral_mint, mm_pool_market_pubkey)| {
                    if !collateral_mint
                        .eq(&Pubkey::from_str("11111111111111111111111111111111").unwrap())
                    {
                        println!("MM Pool: {}", collateral_mint);
                        create_collateral_pool(config, mm_pool_market_pubkey, collateral_mint)
                    } else {
                        Ok(PoolPubkeys {
                            pool: Pubkey::from_str("11111111111111111111111111111111").unwrap(),
                            token_account: Pubkey::from_str("11111111111111111111111111111111")
                                .unwrap(),
                        })
                    }
                })
                .collect::<Result<Vec<PoolPubkeys>, ClientError>>()?;
            collateral_mints
                .iter()
                .map(|(collateral_mint, _mm_pool_market_pubkey)| {
                    if !collateral_mint
                        .eq(&Pubkey::from_str("11111111111111111111111111111111").unwrap())
                    {
                        create_transit(config, depositor_pubkey, collateral_mint, None)
                    } else {
                        Ok(Pubkey::from_str("11111111111111111111111111111111").unwrap())
                    }
                })
                .collect::<Result<Vec<Pubkey>, ClientError>>()?;

            let collateral_pools = collateral_mints
                .iter()
                .zip(mm_pool_collection)
                .map(
                    |((collateral_mint, _mm_pool_market_pubkey), mm_pool_pubkeys)| {
                        CollateralPoolAccounts {
                            pool: mm_pool_pubkeys.pool,
                            pool_token_account: mm_pool_pubkeys.token_account,
                            token_mint: *collateral_mint,
                        }
                    },
                )
                .collect();

            let mut accounts = pair.1;
            accounts.collateral_pools = collateral_pools;
        }
        initialized_accounts
            .save(config.accounts_path.as_str())
            .unwrap();
        Ok(())
    }
}
