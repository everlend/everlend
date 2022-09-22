use crate::liquidity_mining::quarry_raw_test::create_miner;
use crate::utils::{arg, init_token_account};
use crate::{Config, ToolkitCommand};
use clap::{Arg, ArgMatches};
use solana_clap_utils::input_parsers::value_of;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;

const ARG_TOKEN: &str = "token";

#[derive(Clone, Copy)]
pub struct InitQuarryMiningAccountsCommand;

impl<'a> ToolkitCommand<'a> for InitQuarryMiningAccountsCommand {
    fn get_name(&self) -> &'a str {
        "init-quarry-mining-accounts"
    }

    fn get_description(&self) -> &'a str {
        "Init Quarry mining accounts"
    }

    fn get_args(&self) -> Vec<Arg<'a, 'a>> {
        vec![arg(ARG_TOKEN, true)
            .short("t")
            .help("Token")
            .value_name("TOKEN")]
    }

    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>> {
        vec![]
    }

    fn handle(&self, config: &Config, arg_matches: Option<&ArgMatches>) -> anyhow::Result<()> {
        let arg_matches = arg_matches.unwrap();
        let token = value_of::<String>(arg_matches, ARG_TOKEN).unwrap();

        let default_accounts = config.get_default_accounts();
        let mut initialized_accounts = config.get_initialized_accounts();
        let quarry_mining = initialized_accounts.quarry_mining.get_mut(&token).unwrap();
        let miner_vault = Keypair::new();
        create_miner(config, &miner_vault)?;
        quarry_mining.miner_vault = miner_vault.pubkey();
        println!("miner vault {}", miner_vault.pubkey());
        let token_source = Keypair::new();
        init_token_account(config, &token_source, &default_accounts.quarry.token_mint)?;
        quarry_mining.token_source = token_source.pubkey();
        println!("token source {}", token_source.pubkey());
        let rewards_account = Keypair::new();
        init_token_account(
            config,
            &rewards_account,
            &default_accounts.quarry.rewards_token_mint,
        )?;
        quarry_mining.rewards_token_account = rewards_account.pubkey();
        println!("rewards token account {}", rewards_account.pubkey());
        let fee_account = Keypair::new();
        init_token_account(
            config,
            &fee_account,
            &default_accounts.quarry.rewards_token_mint,
        )?;
        quarry_mining.fee_token_account = fee_account.pubkey();
        println!("fee token account {}", fee_account.pubkey());
        initialized_accounts.save(config.accounts_path.as_str())?;
        Ok(())
    }
}
