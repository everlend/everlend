use crate::helpers::migrate_depositor;
use crate::{utils::{Config, arg_amount, arg_pubkey}, ToolkitCommand};
use clap::{Arg, ArgMatches};
use solana_clap_utils::input_parsers::{value_of, pubkey_of};

const ARG_AMOUNT: &str = "amount-to-distribute";
const ARG_TOKEN_MINT: &str = "token-mint";
pub struct MigrateDepositorCommand;

impl<'a> ToolkitCommand<'a> for MigrateDepositorCommand {
    fn get_name(&self) -> &'a str {
        "depositor"
    }

    fn get_description(&self) -> &'a str {
        "Migrate Rebalnce account"
    }

    fn get_args(&self) -> Vec<Arg<'a, 'a>> {
        vec![
            arg_pubkey(ARG_TOKEN_MINT, true).help("Token mint pubkey"),
            arg_amount(ARG_AMOUNT, true).help("Amount to distribute"),
        ]
    }

    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>> {
        vec![]
    }

    fn handle(&self, config: &Config, arg_matches: Option<&ArgMatches>) -> anyhow::Result<()> {
        println!("Started Depositor migration");
        let arg_matches = arg_matches.unwrap();
        let acc = config.get_initialized_accounts();
        let amount_to_distribute = value_of::<u64>(arg_matches, ARG_AMOUNT).unwrap();
        let token_mint_pubkey = pubkey_of(arg_matches, ARG_REBALANCING).unwrap();

        migrate_depositor(config, &acc.depositor, &acc.registry, &token_mint_pubkey, amount_to_distribute)?;

        Ok(())
    }
}
