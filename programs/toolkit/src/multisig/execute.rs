use clap::{Arg, ArgMatches};
use solana_clap_utils::input_parsers::pubkey_of;
use crate::{Config, ToolkitCommand};
use crate::helpers::execute_transaction;
use crate::utils::arg_pubkey;

const ARG_TRANSACTION: &str = "transaction";
const ARG_MULTISIG: &str = "multisig";

#[derive(Clone, Copy)]
pub struct ExecuteCommand;

impl<'a> ToolkitCommand<'a> for ExecuteCommand {
    fn get_name(&self) -> &'a str {
        return "execute";
    }

    fn get_description(&self) -> &'a str {
        return "Execute transaction";
    }

    fn get_args(&self) -> Vec<Arg<'a, 'a>> {
        return vec![
            arg_pubkey(ARG_TRANSACTION, true).help("Transaction account pubkey").short("tx"),
            arg_pubkey(ARG_MULTISIG, true).help("Multisig pubkey"),
        ]
    }

    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>> {
        return vec![];
    }

    fn handle(&self, config: &Config, arg_matches: Option<&ArgMatches>) -> anyhow::Result<()> {
        let arg_matches = arg_matches.unwrap();
        let transaction_pubkey = pubkey_of(arg_matches, ARG_TRANSACTION).unwrap();
        let multisig_pubkey = pubkey_of(arg_matches, ARG_MULTISIG).unwrap();

        println!("transaction_pubkey = {:#?}", transaction_pubkey);
        println!("multisig_pubkey = {:?}", multisig_pubkey);

        let signature = execute_transaction(config, &multisig_pubkey, &transaction_pubkey)?;

        println!("signature = {:?}", signature);

        Ok(())
    }
}