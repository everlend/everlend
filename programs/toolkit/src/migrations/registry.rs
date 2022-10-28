use crate::helpers::migrate_registry;
use crate::{Config, ToolkitCommand};
use clap::{Arg, ArgMatches};
use everlend_registry::state::{MoneyMarket, MoneyMarkets};

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
        money_markets[0] = MoneyMarket {
            id: everlend_utils::integrations::MoneyMarket::PortFinance,
            program_id: default_accounts.port_finance[0].program_id,
            lending_market: default_accounts.port_finance[0].lending_market,
        };
        money_markets[1] = MoneyMarket {
            id: everlend_utils::integrations::MoneyMarket::Larix,
            program_id: default_accounts.larix[0].program_id,
            lending_market: default_accounts.larix[0].lending_market,
        };
        money_markets[2] = MoneyMarket {
            id: everlend_utils::integrations::MoneyMarket::Solend,
            program_id: default_accounts.solend[0].program_id,
            lending_market: default_accounts.solend[0].lending_market,
        };
        money_markets[3] = MoneyMarket {
            id: everlend_utils::integrations::MoneyMarket::Tulip,
            program_id: default_accounts.tulip[0].program_id,
            lending_market: default_accounts.tulip[0].lending_market,
        };
        money_markets[4] = MoneyMarket {
            id: everlend_utils::integrations::MoneyMarket::Francium,
            program_id: default_accounts.francium[0].program_id,
            lending_market: default_accounts.francium[0].lending_market,
        };

        migrate_registry(config, &initialized_accounts.registry, money_markets)?;

        Ok(())
    }
}
