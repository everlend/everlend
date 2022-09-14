use crate::helpers::create_income_pool_safety_fund_token_account;
use crate::utils::arg;
use crate::{Config, ToolkitCommand};
use clap::{Arg, ArgMatches};
use solana_clap_utils::input_parsers::value_of;

const ARG_CASE: &str = "case";

#[derive(Clone, Copy)]
pub struct CreateSafetyFundTokenAccountCommand;

impl<'a> ToolkitCommand<'a> for CreateSafetyFundTokenAccountCommand {
    fn get_name(&self) -> &'a str {
        "create-safety-fund-token-account"
    }

    fn get_description(&self) -> &'a str {
        "Run create income pool safety fund token account"
    }

    fn get_args(&self) -> Vec<Arg<'a, 'a>> {
        vec![arg(ARG_CASE, true).value_name("NAME").index(1).help("Case")]
    }

    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>> {
        vec![]
    }

    fn handle(&self, config: &Config, arg_matches: Option<&ArgMatches>) -> anyhow::Result<()> {
        let arg_matches = arg_matches.unwrap();

        let case = value_of::<String>(arg_matches, "case").unwrap();

        let initialiazed_accounts = config.get_initialized_accounts();

        let token = initialiazed_accounts.token_accounts.get(&case).unwrap();

        println!("Create income pool safety fund token account");
        create_income_pool_safety_fund_token_account(
            config,
            &initialiazed_accounts.income_pool_market,
            &token.mint,
        )?;
        println!("Finished!");

        Ok(())
    }
}
