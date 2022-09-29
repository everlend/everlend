use crate::utils::arg_pubkey;
use crate::{Config, ToolkitCommand};
use anchor_lang::prelude::Pubkey;
use clap::{Arg, ArgMatches};
use everlend_rewards::state::RewardPool;
use solana_clap_utils::input_parsers::pubkey_of;

const ARG_MINT: &str = "mint";

#[derive(Clone, Copy)]
pub struct GetPoolCommand;

impl<'a> ToolkitCommand<'a> for GetPoolCommand {
    fn get_name(&self) -> &'a str {
        "get-pool"
    }

    fn get_description(&self) -> &'a str {
        "Get rebalancing account"
    }

    fn get_args(&self) -> Vec<Arg<'a, 'a>> {
        vec![arg_pubkey(ARG_MINT, true).help("Token mint")]
    }

    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>> {
        vec![]
    }

    fn handle(&self, config: &Config, arg_matches: Option<&ArgMatches>) -> anyhow::Result<()> {
        let arg_matches = arg_matches.unwrap();
        let mint = pubkey_of(arg_matches, ARG_MINT).unwrap();
        let acc = config.get_initialized_accounts();

        let (reward_pool_pubkey, _) = everlend_rewards::find_reward_pool_program_address(
            &everlend_rewards::id(),
            &acc.rewards_root,
            &mint,
        );

        let account: RewardPool = config.get_account_unpack(&reward_pool_pubkey)?;
        println!("{:#?}", reward_pool_pubkey);
        println!("{:#?}", account);

        account.vaults.iter().for_each(|v| {
            let vault_seeds = &[
                b"vault".as_ref(),
                &reward_pool_pubkey.to_bytes()[..32],
                &v.reward_mint.to_bytes()[..32],
                &[v.bump],
            ];

            let pkey =
                Pubkey::create_program_address(vault_seeds, &everlend_rewards::id()).unwrap();

            let a = config.rpc_client.get_account(&pkey).unwrap();
            println!("Vault {}: Reward mint: {}", pkey, v.reward_mint);
            println!("{:#?}", a);
        });

        Ok(())
    }
}
