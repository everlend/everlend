use crate::{depositor, utils::Config, ToolkitCommand};
use anyhow::bail;
use clap::{Arg, ArgMatches};
use everlend_registry::{find_config_program_address, state::DeprecatedRegistryConfig};
use solana_program::program_pack::Pack;

pub struct MigrateDepositorCommand;

impl<'a> ToolkitCommand<'a> for MigrateDepositorCommand {
    fn get_name(&self) -> &'a str {
        return "depositor";
    }

    fn get_description(&self) -> &'a str {
        return "migrate depositor";
    }

    fn get_args(&self) -> Vec<Arg<'a, 'a>> {
        return vec![];
    }

    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>> {
        return vec![];
    }

    fn handle(&self, config: &Config, arg_matches: Option<&ArgMatches>) -> anyhow::Result<()> {
        let arg_matches = arg_matches.unwrap();

        println!("Started Depositor migration");
        // Check that RegistryConfig migrated
        {
            let (registry_config_pubkey, _) = find_config_program_address(
                &everlend_registry::id(),
                &config.initialized_accounts.registry,
            );
            let account = config.rpc_client.get_account(&registry_config_pubkey)?;
            if DeprecatedRegistryConfig::unpack_unchecked(&account.data).is_ok() {
                bail!("RegistryConfig is not migrated yet.")
            }
        }

        for (_, token_accounts) in config.initialized_accounts.token_accounts {
            println!("Mint {}", &token_accounts.mint);
            depositor::migrate_depositor(
                config,
                &config.initialized_accounts.depositor,
                &config.initialized_accounts.registry,
                &token_accounts.mint,
            )?;
        }

        Ok(())
    }
}
