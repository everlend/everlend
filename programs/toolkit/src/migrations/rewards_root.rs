use crate::{utils::Config, ToolkitCommand};
use clap::{Arg, ArgMatches};
use everlend_rewards::state::RewardsRoot;
use solana_sdk::transaction::Transaction;

pub struct MigrateRewardsRootCommand;

impl<'a> ToolkitCommand<'a> for MigrateRewardsRootCommand {
    fn get_name(&self) -> &'a str {
        "rewards-root"
    }

    fn get_description(&self) -> &'a str {
        "Migrate rewards-root account"
    }

    fn get_args(&self) -> Vec<Arg<'a, 'a>> {
        vec![]
    }

    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>> {
        vec![]
    }

    fn handle(&self, config: &Config, _arg_matches: Option<&ArgMatches>) -> anyhow::Result<()> {
        let acc = config.get_initialized_accounts();

        let tx = Transaction::new_with_payer(
            &[everlend_rewards::instruction::migrate_root(
                &everlend_rewards::id(),
                &acc.rewards_root,
                &config.fee_payer.pubkey(),
            )],
            Some(&config.fee_payer.pubkey()),
        );

        config.sign_and_send_and_confirm_transaction(tx, vec![config.fee_payer.as_ref()])?;

        let r: RewardsRoot = config.get_account_unpack(&acc.rewards_root)?;
        println!("Migration of rewards root: \n{:?}", &r);

        Ok(())
    }
}
