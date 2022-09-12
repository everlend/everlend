use std::str::FromStr;
use clap::{Arg, ArgMatches};
use solana_clap_utils::input_parsers::value_of;
use solana_program::pubkey::Pubkey;
use crate::{Config, ToolkitCommand};
use crate::helpers::create_multisig;
use crate::utils::{arg, arg_multiple};

const ARG_OWNERS: &str = "owners";
const ARG_THRESHOLD: &str = "threshold";

#[derive(Clone, Copy)]
pub struct CreateMultisigCommand;

impl<'a> ToolkitCommand<'a> for CreateMultisigCommand {
    fn get_name(&self) -> &'a str {
        return "create";
    }

    fn get_description(&self) -> &'a str {
        return "Create a new multisig";
    }

    fn get_args(&self) -> Vec<Arg<'a, 'a>> {
        return vec![
            arg_multiple(ARG_OWNERS, true),
            arg(ARG_THRESHOLD, true).short("th").value_name("NUMBER").help("Threshold"),
        ];
    }

    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>> {
        return vec![];
    }

    fn handle(&self, config: &Config, arg_matches: Option<&ArgMatches>) -> anyhow::Result<()> {
        let arg_matches = arg_matches.unwrap();

        let owners: Vec<_> = arg_matches
            .values_of(ARG_OWNERS)
            .unwrap()
            .map(|str| Pubkey::from_str(str).unwrap())
            .collect();
        let threshold = value_of::<u64>(arg_matches, ARG_THRESHOLD).unwrap();

        println!("owners = {:#?}", owners);
        println!("owners = {:#?}", owners);
        println!("threshold = {:?}", threshold);

       let (multisig_pubkey, multisig_pda) =
            create_multisig(config, None, owners, threshold)?;

        println!("multisig_pubkey = {:?}", multisig_pubkey);
        println!("multisig_pda = {:?}", multisig_pda);

        Ok(())
    }
}