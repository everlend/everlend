use crate::helpers::migrate_depositor;
use crate::utils::arg_pubkey;
use crate::{utils::Config, ToolkitCommand};
use clap::{Arg, ArgMatches};
use solana_clap_utils::input_parsers::pubkey_of;

const ARG_SOURCE: &str = "source";
const ARG_TARGET: &str = "target";

pub struct MigrateDepositorCommand;

impl<'a> ToolkitCommand<'a> for MigrateDepositorCommand {
    fn get_name(&self) -> &'a str {
        "depositor"
    }

    fn get_description(&self) -> &'a str {
        "Migrate Depositor account. Must be invoke after migrate-registry-config."
    }

    fn get_args(&self) -> Vec<Arg<'a, 'a>> {
        vec![
            arg_pubkey(ARG_SOURCE, true)
                .help("Old registry")
                .value_name("SOURCE"),
            arg_pubkey(ARG_TARGET, true)
                .help("New registry")
                .value_name("TARGET"),
        ]
    }

    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>> {
        vec![]
    }

    fn handle(&self, config: &Config, arg_matches: Option<&ArgMatches>) -> anyhow::Result<()> {
        println!("Started Depositor migration");
        let arg_matches = arg_matches.unwrap();
        let source = pubkey_of(arg_matches, ARG_SOURCE).unwrap();
        let target = pubkey_of(arg_matches, ARG_TARGET).unwrap();
        let initialized_accounts = config.get_initialized_accounts();

        migrate_depositor(config, &initialized_accounts.depositor, &source, &target)?;

        Ok(())
    }
}
