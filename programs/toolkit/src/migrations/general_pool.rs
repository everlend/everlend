use crate::helpers::migrate_general_pool_account;
use crate::utils::arg;
use crate::{utils::Config, InitializedAccounts, ToolkitCommand};
use clap::{Arg, ArgMatches};
use solana_clap_utils::input_parsers::value_of;

const ARG_CASE: &str = "case";

pub struct MigrateGeneralPoolCommand;

impl<'a> ToolkitCommand<'a> for MigrateGeneralPoolCommand {
    fn get_name(&self) -> &'a str {
        "general-pool"
    }

    fn get_description(&self) -> &'a str {
        "migrate general pool"
    }

    fn get_args(&self) -> Vec<Arg<'a, 'a>> {
        vec![arg(ARG_CASE, true).value_name("TOKEN").help("Case")]
    }

    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>> {
        vec![]
    }

    fn handle(&self, config: &Config, arg_matches: Option<&ArgMatches>) -> anyhow::Result<()> {
        let arg_matches = arg_matches.unwrap();
        let accounts_path = arg_matches.value_of("accounts").unwrap_or("accounts.yaml");
        let case = value_of::<String>(arg_matches, "case");
        let initialiazed_accounts = InitializedAccounts::load(accounts_path).unwrap_or_default();

        if case.is_none() {
            println!("Migrate token mint not presented");
            return Ok(());
        }

        let _token = initialiazed_accounts
            .token_accounts
            .get(&case.unwrap())
            .unwrap();

        println!("Migrate withdraw requests");
        migrate_general_pool_account(config)?;
        println!("Finished!");

        Ok(())
    }
}
