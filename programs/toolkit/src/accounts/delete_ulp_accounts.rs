use crate::helpers::{
    bulk_delete_pool_borrow_authorities, bulk_delete_pools, delete_pool_market,
    fetch_pool_authorities, fetch_pools,
};
use crate::utils::arg_pubkey;
use crate::{Config, ToolkitCommand};
use clap::{Arg, ArgMatches};
use everlend_ulp::state::PoolMarket;
use solana_clap_utils::input_parsers::pubkey_of;
use solana_program::program_pack::Pack;

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
        vec![arg_pubkey(ARG_MARKET, true).help("Market")]
    }

    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>> {
        vec![]
    }

    fn handle(&self, config: &Config, arg_matches: Option<&ArgMatches>) -> anyhow::Result<()> {
        let arg_matches = arg_matches.unwrap();

        let market_pubkey = pubkey_of(arg_matches, ARG_MARKET).unwrap();
        let market_account = config.rpc_client.get_account(&market_pubkey).unwrap();
        let market = PoolMarket::unpack(&market_account.data).unwrap();
        println!("Market:");
        println!("{:#?}", market);

        let pools = fetch_pools(config, &market_pubkey);
        println!("Pools:");
        println!("{:#?}", pools);

        for (pool_pubkey, _) in &pools {
            let pool_authorities = fetch_pool_authorities(config, pool_pubkey);
            println!("Pool borrow authorities:");
            println!("{:#?}", pool_authorities);

            bulk_delete_pool_borrow_authorities(
                config,
                &market_pubkey,
                pool_pubkey,
                &pool_authorities,
            )
            .unwrap();
        }

        bulk_delete_pools(config, &market_pubkey, &pools).unwrap();

        println!("Deleting pool market: {:#?}", market_pubkey);
        delete_pool_market(config, &market_pubkey).unwrap();

        Ok(())
    }
}
