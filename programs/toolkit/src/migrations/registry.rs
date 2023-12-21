use crate::helpers::migrate_registry;
use crate::{Config, ToolkitCommand};
use clap::{Arg, ArgMatches};
use everlend_registry::state::MoneyMarkets;

pub struct MigrateRegistryCommand;

impl<'a> ToolkitCommand<'a> for MigrateRegistryCommand {
    fn get_name(&self) -> &'a str {
        "registry"
    }

    fn get_description(&self) -> &'a str {
        "Migrate registry"
    }

    fn get_args(&self) -> Vec<Arg<'a, 'a>> {
        vec![]
    }

    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>> {
        vec![]
    }

    fn handle(&self, config: &Config, _arg_matches: Option<&ArgMatches>) -> anyhow::Result<()> {
        println!("Started Depositor migration");

        let default_accounts = config.get_default_accounts();
        let initialized_accounts = config.get_initialized_accounts();

        let mut money_markets = MoneyMarkets::default();
        money_markets[0] = everlend_utils::integrations::MoneyMarket::PortFinance {
            money_market_program_id: default_accounts.port_finance.program_id,
        };
        money_markets[1] = everlend_utils::integrations::MoneyMarket::Larix {
            money_market_program_id: default_accounts.larix.program_id,
        };
        money_markets[2] = everlend_utils::integrations::MoneyMarket::Solend {
            money_market_program_id: default_accounts.solend.program_id,
            lending_market: default_accounts.solend.lending_market,
        };
        money_markets[3] = everlend_utils::integrations::MoneyMarket::Tulip {
            money_market_program_id: default_accounts.tulip.program_id,
        };
        money_markets[4] = everlend_utils::integrations::MoneyMarket::Francium {
            money_market_program_id: default_accounts.francium.program_id,
        };

        migrate_registry(config, &initialized_accounts.registry, money_markets)?;

        Ok(())
    }
}
