use crate::helpers::reset_rebalancing;
use crate::utils::{arg_amount, arg_multiple, arg_pubkey};
use crate::{Config, ToolkitCommand};
use clap::{Arg, ArgMatches};
use everlend_depositor::state::Rebalancing;
use everlend_liquidity_oracle::state::DistributionArray;
use solana_clap_utils::input_parsers::{pubkey_of, value_of, values_of};

const ARG_REBALANCING: &str = "rebalancing";
const ARG_AMOUNT: &str = "amount-to-distribute";
const ARG_DISTRIBUTED_LIQUIDITY: &str = "distributed-liquidity";
const ARG_DISTRIBUTION: &str = "distribution";

#[derive(Clone, Copy)]
pub struct ResetRebalancingCommand;

impl<'a> ToolkitCommand<'a> for ResetRebalancingCommand {
    fn get_name(&self) -> &'a str {
        "reset-rebalancing"
    }

    fn get_description(&self) -> &'a str {
        "Reset rebalancing"
    }

    fn get_args(&self) -> Vec<Arg<'a, 'a>> {
        vec![
            arg_pubkey(ARG_REBALANCING, true).help("Rebalancing pubkey"),
            arg_amount(ARG_AMOUNT, true).help("Amount to distribute"),
            arg_multiple(ARG_DISTRIBUTED_LIQUIDITY, true).help("Distributed liduidity"),
            arg_multiple(ARG_DISTRIBUTION, true)
                .value_name("DISTRIBUTION")
                .short("d")
                .number_of_values(10),
        ]
    }

    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>> {
        vec![]
    }

    fn handle(&self, config: &Config, arg_matches: Option<&ArgMatches>) -> anyhow::Result<()> {
        let arg_matches = arg_matches.unwrap();
        let rebalancing_pubkey = pubkey_of(arg_matches, ARG_REBALANCING).unwrap();
        let amount_to_distribute = value_of::<u64>(arg_matches, ARG_AMOUNT).unwrap();
        let liquidity: Vec<u64> = values_of::<u64>(arg_matches, ARG_DISTRIBUTED_LIQUIDITY).unwrap();
        let distribution: Vec<u64> = values_of::<u64>(arg_matches, ARG_DISTRIBUTION).unwrap();
        let initialiazed_accounts = config.get_initialized_accounts();

        let rebalancing = config.get_account_unpack::<Rebalancing>(&rebalancing_pubkey)?;
        let mut distributed_liquidity = DistributionArray::default();
        distributed_liquidity.copy_from_slice(liquidity.as_slice());

        let mut distribution_array = DistributionArray::default();
        distribution_array.copy_from_slice(distribution.as_slice());

        println!("distribution_array {:?}", distribution_array);

        reset_rebalancing(
            config,
            &initialiazed_accounts.registry,
            &rebalancing.depositor,
            &rebalancing.mint,
            amount_to_distribute,
            distributed_liquidity,
            distribution_array,
        )?;

        Ok(())
    }
}
