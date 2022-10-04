use crate::{utils::Config, ToolkitCommand};
use clap::{Arg, ArgMatches};
use solana_program::instruction::Instruction;
use solana_sdk::transaction::Transaction;

pub struct MigrateRewardsPoolCommand;

impl<'a> ToolkitCommand<'a> for MigrateRewardsPoolCommand {
    fn get_name(&self) -> &'a str {
        "rewards-pools"
    }

    fn get_description(&self) -> &'a str {
        "Migrate rewards-pool accounts"
    }

    fn get_args(&self) -> Vec<Arg<'a, 'a>> {
        vec![]
    }

    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>> {
        vec![]
    }

    fn handle(&self, config: &Config, _arg_matches: Option<&ArgMatches>) -> anyhow::Result<()> {
        let acc = config.get_initialized_accounts();

        let ix: Vec<Instruction> = acc
            .token_accounts
            .iter()
            .map(|(_, token)| -> Instruction {
                let (reward_pool, _) = everlend_rewards::find_reward_pool_program_address(
                    &everlend_rewards::id(),
                    &acc.rewards_root,
                    &token.mint,
                );

                everlend_rewards::instruction::migrate_pool(
                    &everlend_rewards::id(),
                    &acc.rewards_root,
                    &reward_pool,
                    &config.fee_payer.pubkey(),
                    &token.mint,
                )
            })
            .collect();

        let tx = Transaction::new_with_payer(&ix, Some(&config.fee_payer.pubkey()));

        let res =
            config.sign_and_send_and_confirm_transaction(tx, vec![config.fee_payer.as_ref()])?;

        println!("{}", res);

        Ok(())
    }
}
