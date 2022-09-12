use crate::helpers::init_depositor;
use crate::utils::arg_pubkey;
use crate::{arg_keypair, Config, ToolkitCommand};
use clap::{Arg, ArgMatches};
use solana_clap_utils::input_parsers::{keypair_of, pubkey_of};

const ARG_KEYPAIR: &str = "keypair";
const ARG_REBALANCE_EXECUTOR: &str = "rebalance-executor";

#[derive(Clone, Copy)]
pub struct CreateDepositorCommand;

impl<'a> ToolkitCommand<'a> for CreateDepositorCommand {
    fn get_name(&self) -> &'a str {
        return "create";
    }

    fn get_description(&self) -> &'a str {
        return "Create a new depositor";
    }

    fn get_args(&self) -> Vec<Arg<'a, 'a>> {
        return vec![
            arg_keypair(ARG_KEYPAIR, false),
            arg_pubkey(ARG_REBALANCE_EXECUTOR, true).help("Rebalance executor pubkey"),
        ];
    }

    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>> {
        return vec![];
    }

    fn handle(&self, config: &Config, arg_matches: Option<&ArgMatches>) -> anyhow::Result<()> {
        let arg_matches = arg_matches.unwrap();
        let keypair = keypair_of(arg_matches, ARG_KEYPAIR);
        let rebalance_executor = pubkey_of(arg_matches, ARG_REBALANCE_EXECUTOR).unwrap();

        let mut initialiazed_accounts = config.get_initialized_accounts();

        let depositor_pubkey = init_depositor(
            config,
            &initialiazed_accounts.registry,
            keypair,
            rebalance_executor,
        )?;

        initialiazed_accounts.depositor = depositor_pubkey;
        initialiazed_accounts.rebalance_executor = rebalance_executor;

        initialiazed_accounts
            .save(&format!("accounts.{}.yaml", config.network))
            .unwrap();

        Ok(())
    }
}
