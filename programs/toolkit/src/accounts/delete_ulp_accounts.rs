use crate::helpers::{
    delete_pool, delete_pool_borrow_authority, delete_pool_market, fetch_pool_authorities,
    fetch_pools,
};
use crate::{arg_keypair, Config, ToolkitCommand};
use clap::{Arg, ArgMatches};
use everlend_ulp::state::PoolMarket;
use solana_clap_utils::input_parsers::keypair_of;
use solana_program::program_pack::Pack;
use solana_sdk::signer::Signer;

const ARG_MARKET: &str = "market";

#[derive(Clone, Copy)]
pub struct DeleteUPLAccountsCommand;

impl<'a> ToolkitCommand<'a> for DeleteUPLAccountsCommand {
    fn get_name(&self) -> &'a str {
        "delete-ulp-accounts"
    }

    fn get_description(&self) -> &'a str {
        "Delete ULP accounts"
    }

    fn get_args(&self) -> Vec<Arg<'a, 'a>> {
        vec![arg_keypair(ARG_MARKET, true)]
    }

    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>> {
        vec![]
    }

    fn handle(&self, config: &Config, arg_matches: Option<&ArgMatches>) -> anyhow::Result<()> {
        let arg_matches = arg_matches.unwrap();

        let market_keypair = keypair_of(arg_matches, ARG_MARKET).unwrap();
        let market_pubkey = &market_keypair.pubkey();
        let market_account = config.rpc_client.get_account(market_pubkey).unwrap();
        let market = PoolMarket::unpack(&market_account.data).unwrap();
        println!("Market:");
        println!("{:#?}", market);

        println!("Pools:");
        let pools = fetch_pools(config, market_pubkey);
        println!("{:#?}", pools);

        for (pool_pubkey, pool) in pools {
            let pool_authorities = fetch_pool_authorities(config, &pool_pubkey);
            println!("Pool borrow authorities:");
            println!("{:#?}", pool_authorities);

            for (borrow_authority_pubkey, _) in pool_authorities {
                println!(
                    "Deleting pool borrow authority: {:#?}",
                    borrow_authority_pubkey
                );

                delete_pool_borrow_authority(
                    config,
                    market_pubkey,
                    &pool_pubkey,
                    &borrow_authority_pubkey,
                )
                .unwrap();
            }

            println!("Deleting pool: {:#?}", pool_pubkey);
            delete_pool(config, market_pubkey, &pool_pubkey, &pool.token_mint).unwrap();
        }

        println!("Deleting pool market: {:#?}", market_keypair.pubkey());
        delete_pool_market(config, &market_keypair).unwrap();

        Ok(())
    }
}
