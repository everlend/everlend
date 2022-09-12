use std::fs;
use clap::{Arg, ArgMatches};
use solana_program_test::{find_file, read_file};
use crate::{Config, ToolkitCommand};
use crate::accounts_config::{DefaultAccounts, save_config_file};
use crate::utils::download_account;
use anchor_lang::AnchorDeserialize;

#[derive(Clone, Copy)]
pub struct SaveQuarryAccountsCommand;

impl<'a> ToolkitCommand<'a> for SaveQuarryAccountsCommand {
    fn get_name(&self) -> &'a str {
        return "save-quarry-accounts";
    }

    fn get_description(&self) -> &'a str {
        return "Save Quarry accounts";
    }

    fn get_args(&self) -> Vec<Arg<'a, 'a>> {
        return vec![];
    }

    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>> {
        return vec![];
    }

    fn handle(&self, config: &Config, _arg_matches: Option<&ArgMatches>) -> anyhow::Result<()> {
        let mut default_accounts = config.get_default_accounts();
        let file_path = "../tests/tests/fixtures/quarry/quarry.bin";

        fs::remove_file(file_path)?;

        println!("quarry {}", default_accounts.quarry.quarry);

        download_account(&default_accounts.quarry.quarry, "quarry", "quarry");

        let data: Vec<u8> = read_file(find_file(file_path).unwrap());
        // first 8 bytes are meta information
        let adjusted = &data[8..];
        let deserialized = quarry_mine::Quarry::try_from_slice(adjusted)?;

        println!("rewarder {}", deserialized.rewarder);
        println!("token mint {}", deserialized.token_mint_key);

        default_accounts.quarry.rewarder = deserialized.rewarder;
        default_accounts.quarry.token_mint = deserialized.token_mint_key;

        save_config_file::<DefaultAccounts, &str>(&default_accounts, "default.devnet.yaml")?;

        Ok(())
    }
}