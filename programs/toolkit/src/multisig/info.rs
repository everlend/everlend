use clap::{Arg, ArgMatches};
use solana_clap_utils::input_parsers::pubkey_of;
use solana_program::pubkey::Pubkey;
use crate::{Config, ToolkitCommand};
use crate::helpers::get_transaction_program_accounts;
use crate::utils::arg_pubkey;
use anchor_lang::AccountDeserialize;

const ARG_MULTISIG: &str = "multisig";

#[derive(Clone, Copy)]
pub struct InfoCommand;

impl<'a> ToolkitCommand<'a> for InfoCommand {
    fn get_name(&self) -> &'a str {
        return "info";
    }

    fn get_description(&self) -> &'a str {
        return "Multisig info";
    }

    fn get_args(&self) -> Vec<Arg<'a, 'a>> {
        return vec![
            arg_pubkey(ARG_MULTISIG, true).help("Multisig pubkey"),
        ]
    }

    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>> {
        return vec![];
    }

    fn handle(&self, config: &Config, arg_matches: Option<&ArgMatches>) -> anyhow::Result<()> {
        let arg_matches = arg_matches.unwrap();
        let multisig_pubkey = pubkey_of(arg_matches, ARG_MULTISIG).unwrap();

        let multisig = config.get_account_deserialize::<serum_multisig::Multisig>(&multisig_pubkey)?;

        println!("Owners: {:?}", multisig.owners);
        println!("Threshold: {:?}", multisig.threshold);

        println!("Transactions:");
        let txs: Vec<(Pubkey, serum_multisig::Transaction)> =
            get_transaction_program_accounts(config, &multisig_pubkey)?
                .into_iter()
                .filter_map(|(address, account)| {
                    let mut data_ref = &account.data[..];
                    match serum_multisig::Transaction::try_deserialize(&mut data_ref) {
                        Ok(tx) => Some((address, tx)),
                        _ => None,
                    }
                })
                .collect();

        for (pubkey, tx) in txs {
            println!("{:?}", pubkey);
            println!("Data: {:?}", tx.data);
            println!("Signers: {:?}", tx.signers);
            println!("Set seqno: {:?}", tx.owner_set_seqno);
            println!("Executed: {:?}", tx.did_execute);
        }

        Ok(())
    }
}