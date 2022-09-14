use crate::helpers::{create_transaction, get_multisig_program_address};
use crate::utils::arg_pubkey;
use crate::{Config, ToolkitCommand};
use clap::{Arg, ArgMatches};
use solana_clap_utils::input_parsers::pubkey_of;
use solana_program::bpf_loader_upgradeable;

const ARG_PROGRAM: &str = "program";
const ARG_BUFFER: &str = "buffer";
const ARG_SPILL: &str = "spill";
const ARG_MULTISIG: &str = "multisig";

#[derive(Clone, Copy)]
pub struct ProposeUpgradeCommand;

impl<'a> ToolkitCommand<'a> for ProposeUpgradeCommand {
    fn get_name(&self) -> &'a str {
        "propose-upgrade"
    }

    fn get_description(&self) -> &'a str {
        "Propose program upgrade"
    }

    fn get_args(&self) -> Vec<Arg<'a, 'a>> {
        vec![
            arg_pubkey(ARG_PROGRAM, true).help("Program pubkey"),
            arg_pubkey(ARG_BUFFER, true).help("Buffer pubkey"),
            arg_pubkey(ARG_SPILL, true).help("Spill pubkey"),
            arg_pubkey(ARG_MULTISIG, true).help("Multisig pubkey"),
        ]
    }

    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>> {
        vec![]
    }

    fn handle(&self, config: &Config, arg_matches: Option<&ArgMatches>) -> anyhow::Result<()> {
        let arg_matches = arg_matches.unwrap();
        let program_pubkey = pubkey_of(arg_matches, ARG_PROGRAM).unwrap();
        let buffer_pubkey = pubkey_of(arg_matches, ARG_BUFFER).unwrap();
        let spill_pubkey = pubkey_of(arg_matches, ARG_SPILL).unwrap();
        let multisig_pubkey = pubkey_of(arg_matches, ARG_MULTISIG).unwrap();

        let default_accounts = config.get_default_accounts();
        let (pda, _) =
            get_multisig_program_address(&default_accounts.multisig_program_id, &multisig_pubkey);

        let upgrade_instruction =
            bpf_loader_upgradeable::upgrade(&program_pubkey, &buffer_pubkey, &pda, &spill_pubkey);

        let transaction_pubkey = create_transaction(config, &multisig_pubkey, upgrade_instruction)?;

        println!("transaction_pubkey = {:?}", transaction_pubkey);

        Ok(())
    }
}
