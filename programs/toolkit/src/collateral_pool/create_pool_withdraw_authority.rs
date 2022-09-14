use crate::helpers::create_pool_withdraw_authority;
use crate::{Config, ToolkitCommand};
use clap::{Arg, ArgMatches};
use everlend_utils::find_program_address;
use solana_client::client_error::ClientError;
use solana_program::pubkey::Pubkey;
use std::str::FromStr;

#[derive(Clone, Copy)]
pub struct CreatePoolWithdrawAuthorityCommand;

impl<'a> ToolkitCommand<'a> for CreatePoolWithdrawAuthorityCommand {
    fn get_name(&self) -> &'a str {
        "create-pool-withdraw-authority"
    }

    fn get_description(&self) -> &'a str {
        "Create pool withdraw authority"
    }

    fn get_args(&self) -> Vec<Arg<'a, 'a>> {
        vec![]
    }

    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>> {
        vec![]
    }

    fn handle(&self, config: &Config, _arg_matches: Option<&ArgMatches>) -> anyhow::Result<()> {
        let mut initialized_accounts = config.get_initialized_accounts();
        let pool_markets = initialized_accounts.collateral_pool_markets;
        let depositor = initialized_accounts.depositor;
        let token_accounts = initialized_accounts.token_accounts.iter_mut();
        for pair in token_accounts {
            pair.1
                .collateral_pools
                .iter()
                .zip(pool_markets.clone())
                .filter(|(keyset, _)| {
                    !keyset
                        .pool
                        .eq(&Pubkey::from_str("11111111111111111111111111111111").unwrap())
                })
                .map(|(keyset, market)| {
                    let (depositor_authority, _) =
                        find_program_address(&everlend_depositor::id(), &depositor);
                    create_pool_withdraw_authority(
                        config,
                        &market,
                        &keyset.pool,
                        &depositor_authority,
                        &config.fee_payer.pubkey(),
                    )
                })
                .collect::<Result<Vec<Pubkey>, ClientError>>()?;
        }
        Ok(())
    }
}
