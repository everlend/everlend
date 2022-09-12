use crate::{utils::Config, ToolkitCommand};
use clap::{Arg, ArgMatches};
use solana_sdk::transaction::Transaction;
use everlend_general_pool::instruction;

pub struct MigrateGeneralPoolCommand;

impl<'a> ToolkitCommand<'a> for MigrateGeneralPoolCommand {
    fn get_name(&self) -> &'a str {
        return "general-pool";
    }

    fn get_description(&self) -> &'a str {
        return "migrate general pool";
    }

    fn get_args(&self) -> Vec<Arg<'a, 'a>> {
        return vec![];
    }

    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>> {
        return vec![];
    }

    fn handle(&self, config: &Config, _arg_matches: Option<&ArgMatches>) -> anyhow::Result<()> {
        let tx = Transaction::new_with_payer(
            &[instruction::migrate_instruction(
                &everlend_general_pool::id(),
            )],
            Some(&config.fee_payer.pubkey()),
        );

        config.sign_and_send_and_confirm_transaction(tx, vec![config.fee_payer.as_ref()])?;

        println!("Finished!");

        Ok(())
    }
}
