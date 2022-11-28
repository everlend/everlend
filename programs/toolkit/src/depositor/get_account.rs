use crate::utils::arg_pubkey;
use crate::{Config, ToolkitCommand};
use clap::{Arg, ArgMatches};
use everlend_depositor::state::Rebalancing;
use everlend_depositor::{RebalancingPDA, TransitPDA};
use everlend_general_pool::state::{Pool, WithdrawalRequests};
use everlend_general_pool::{find_pool_program_address, find_withdrawal_requests_program_address};
use everlend_utils::PDA;
use solana_clap_utils::input_parsers::pubkey_of;
use spl_token::state::Account;

const ARG_MINT: &str = "mint";

#[derive(Clone, Copy)]
pub struct GetRebalancingAccountCommand;

impl<'a> ToolkitCommand<'a> for GetRebalancingAccountCommand {
    fn get_name(&self) -> &'a str {
        "get-rebalancing-account"
    }

    fn get_description(&self) -> &'a str {
        "Get rebalancing account"
    }

    fn get_args(&self) -> Vec<Arg<'a, 'a>> {
        vec![arg_pubkey(ARG_MINT, true).help("Token mint")]
    }

    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>> {
        vec![]
    }

    fn handle(&self, config: &Config, arg_matches: Option<&ArgMatches>) -> anyhow::Result<()> {
        let arg_matches = arg_matches.unwrap();
        let mint = pubkey_of(arg_matches, ARG_MINT).unwrap();
        let acc = config.get_initialized_accounts();

        let (rebalancing_pubkey, _) = RebalancingPDA {
            depositor: acc.depositor.clone(),
            mint,
        }
        .find_address(&everlend_depositor::id());

        let (pool_pubkey, _) = find_pool_program_address(
            &everlend_general_pool::id(),
            &acc.general_pool_market,
            &mint,
        );
        // Check withdrawal requests
        let (withdrawal_requests, _) = find_withdrawal_requests_program_address(
            &everlend_general_pool::id(),
            &acc.general_pool_market,
            &mint,
        );

        let (liquidity_transit, _) = TransitPDA {
            seed: "",
            depositor: acc.depositor.clone(),
            mint,
        }
        .find_address(&everlend_depositor::id());

        let rebalancing: Rebalancing = config.get_account_unpack(&rebalancing_pubkey)?;
        let pool: Pool = config.get_account_unpack(&pool_pubkey)?;
        let pool_ta: Account = config.get_account_unpack(&pool.token_account)?;
        let requests: WithdrawalRequests = config.get_account_unpack(&withdrawal_requests)?;
        let transit: Account = config.get_account_unpack(&liquidity_transit)?;

        println!("{:?}", rebalancing);
        println!("{:?}", pool);
        println!("{:?}", pool_ta);

        println!(
            "total_amount_borrowed: {}. Diff {}. Requests {}. Transit {}",
            pool.total_amount_borrowed,
            rebalancing
                .amount_to_distribute
                .saturating_sub(pool.total_amount_borrowed),
            requests.liquidity_supply,
            transit.amount,
        );

        Ok(())
    }
}
