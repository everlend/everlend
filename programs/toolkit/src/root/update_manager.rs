use crate::helpers::{
    collateral_pool_update_manager, general_pool_update_manager, income_pools_update_manager,
    registry_update_manager,
};
use crate::{
    utils::{arg, arg_keypair, Config},
    ToolkitCommand,
};
use clap::{Arg, ArgMatches};
use solana_clap_utils::input_parsers::keypair_of;

const ARG_SOURCE: &str = "source";
const ARG_TARGET: &str = "target";
const ARG_PROGRAM: &str = "program";

#[derive(Clone, Copy)]
pub struct UpdateManagerCommand;

impl<'a> ToolkitCommand<'a> for UpdateManagerCommand {
    fn get_name(&self) -> &'a str {
        return "update-manager";
    }

    fn get_description(&self) -> &'a str {
        return "Update pool manager account";
    }

    fn get_args(&self) -> Vec<Arg<'a, 'a>> {
        return vec![
            arg_keypair(ARG_SOURCE, true).help("Old manager keypair"),
            arg_keypair(ARG_TARGET, true).help("New manager keypair"),
            arg(ARG_PROGRAM, true).help(
                "Program to update manager: collateral-pool|general-pool|income-pools|registry|ulp",
            ),
        ];
    }

    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>> {
        return vec![];
    }

    fn handle(&self, config: &Config, arg_matches: Option<&ArgMatches>) -> anyhow::Result<()> {
        let arg_matches = arg_matches.unwrap();
        let source = keypair_of(arg_matches, ARG_SOURCE).unwrap();
        let target = keypair_of(arg_matches, ARG_TARGET).unwrap();
        let program = arg_matches.value_of(ARG_PROGRAM).unwrap();

        match program {
            "collateral-pool" => {
                for p in config.initialized_accounts.collateral_pool_markets.iter() {
                    println!("Updating collateral pool manager: Pool market: {}", p);
                    collateral_pool_update_manager(config, p, &source, &target)?;
                }
            }
            "general-pool" => {
                println!(
                    "Updating general pool manager: Market {}",
                    config.initialized_accounts.general_pool_market
                );
                general_pool_update_manager(
                    config,
                    &config.initialized_accounts.general_pool_market,
                    &source,
                    &target,
                )?;
            }
            "income-pools" => {
                println!(
                    "Updating income pool manager: Market {}",
                    config.initialized_accounts.income_pool_market
                );
                income_pools_update_manager(
                    config,
                    &config.initialized_accounts.income_pool_market,
                    &source,
                    &target,
                )?;
            }
            "registry" => {
                println!(
                    "Updating registry manager: Registry {}",
                    config.initialized_accounts.registry
                );
                registry_update_manager(
                    config,
                    &config.initialized_accounts.registry,
                    &source,
                    &target,
                )?;
            }
            _ => {
                return Err(anyhow::anyhow!("wrong program"));
            }
        }
        println!("Program {:?}", program);

        Ok(())
    }
}
