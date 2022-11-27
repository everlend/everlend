use crate::helpers::{bulk_migrate_pool_borrow_authority, bulk_migrate_pool_withdraw_authority};
use crate::utils::get_asset_maps;
use crate::{utils::Config, ToolkitCommand};
use clap::{Arg, ArgMatches};
use everlend_collateral_pool::find_pool_program_address;
use everlend_collateral_pool::state::{PoolBorrowAuthority, PoolWithdrawAuthority};
use everlend_utils::find_program_address;
use solana_client::client_error::ClientError;
use solana_program::pubkey::Pubkey;

pub struct MigrateCollateralPoolCommand;

impl<'a> ToolkitCommand<'a> for MigrateCollateralPoolCommand {
    fn get_name(&self) -> &'a str {
        "collateral-pool"
    }

    fn get_description(&self) -> &'a str {
        "Migrate Collateral pool authorities"
    }

    fn get_args(&self) -> Vec<Arg<'a, 'a>> {
        vec![]
    }

    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>> {
        vec![]
    }

    fn handle(&self, config: &Config, _arg_matches: Option<&ArgMatches>) -> anyhow::Result<()> {
        println!("Started Collateral pool withdraw authorities migration");
        let default_accounts = config.get_default_accounts();
        let initialiazed_accounts = config.get_initialized_accounts();
        let (_, collateral_mint_map) = get_asset_maps(default_accounts);

        let mut withdraw_authorities = vec![];
        let mut borrow_authorities = vec![];

        for (_, mm_collateral_mint) in collateral_mint_map.iter() {
            for (money_market_index, collateral_mint) in mm_collateral_mint.iter().enumerate() {
                if collateral_mint.is_none() {
                    continue
                }
                let collateral_mint = collateral_mint.unwrap();
                let collateral_pool_market_pubkey =
                    initialiazed_accounts.collateral_pool_markets[money_market_index];

                let (pool_pubkey, _) = find_pool_program_address(
                    &everlend_collateral_pool::id(),
                    &collateral_pool_market_pubkey,
                    &collateral_mint,
                );

                let (depositor_authority, _) = find_program_address(
                    &everlend_depositor::id(),
                    &initialiazed_accounts.depositor,
                );

                let (pool_withdraw_authority_pubkey, _) = Pubkey::find_program_address(
                    &[&pool_pubkey.to_bytes(), &depositor_authority.to_bytes()],
                    &everlend_collateral_pool::id(),
                );

                if config
                    .rpc_client
                    .get_balance(&pool_withdraw_authority_pubkey)
                    .unwrap()
                    == 0
                {
                    continue;
                }

                let pool_withdraw_authority: Result<PoolWithdrawAuthority, ClientError> =
                    config.get_account_unpack(&pool_withdraw_authority_pubkey);

                match pool_withdraw_authority {
                    Ok(pool_withdraw_authority) => {
                        withdraw_authorities.push((
                            collateral_pool_market_pubkey,
                            pool_withdraw_authority_pubkey,
                            pool_withdraw_authority,
                        ));
                    }
                    Err(_) => {
                        let pool_borrow_authority: PoolBorrowAuthority = config
                            .get_account_unpack(&pool_withdraw_authority_pubkey)
                            .unwrap();
                        borrow_authorities.push((
                            collateral_pool_market_pubkey,
                            pool_withdraw_authority_pubkey,
                            pool_borrow_authority,
                        ))
                    }
                }
            }
        }

        let withdraw_authorities_chunks = withdraw_authorities.chunks(5);
        for chunk in withdraw_authorities_chunks {
            bulk_migrate_pool_withdraw_authority(config, chunk).unwrap();
        }

        let borrow_authorities_chunks = borrow_authorities.chunks(10);
        for chunk in borrow_authorities_chunks {
            bulk_migrate_pool_borrow_authority(config, chunk).unwrap();
        }

        Ok(())
    }
}
