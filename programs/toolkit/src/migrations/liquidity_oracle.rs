use crate::{utils::Config, ToolkitCommand};
use clap::{Arg, ArgMatches};
use everlend_liquidity_oracle::instruction;
use solana_sdk::transaction::Transaction;
pub struct MigrateLiquidityOracleCommand;

impl<'a> ToolkitCommand<'a> for MigrateLiquidityOracleCommand {
    fn get_name(&self) -> &'a str {
        "liquidity-oracle"
    }

    fn get_description(&self) -> &'a str {
        "Migrate PoolOracle account"
    }

    fn get_args(&self) -> Vec<Arg<'a, 'a>> {
        vec![]
    }

    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>> {
        vec![]
    }

    fn handle(&self, config: &Config, _arg_matches: Option<&ArgMatches>) -> anyhow::Result<()> {
        println!("Started LiquidityOracle migration");
        let acc = config.get_initialized_accounts();

        for token in acc.token_accounts {
            let tx = Transaction::new_with_payer(
                &[instruction::migrate(
                    &everlend_liquidity_oracle::id(),
                    &acc.liquidity_oracle,
                    &config.fee_payer.pubkey(),
                    &token.1.mint,
                )],
                Some(&config.fee_payer.pubkey()),
            );

            config.sign_and_send_and_confirm_transaction(tx, vec![config.fee_payer.as_ref()])?;
        }

        Ok(())
    }
}
