use crate::helpers::{
    deposit as helper_deposit, depositor_deposit, depositor_withdraw,
    get_withdrawal_request_accounts, start_rebalancing as helper_start_rebalancing,
    start_rebalancing, update_liquidity_distribution, withdraw as helper_withdraw,
    withdraw_request,
};
use crate::utils::{arg, delay};
use crate::{distribution, Config, InitializedAccounts, ToolkitCommand};
use anyhow::Context;
use clap::{Arg, ArgMatches};
use everlend_depositor::find_rebalancing_program_address;
use everlend_depositor::state::{Rebalancing, RebalancingOperation};
use everlend_general_pool::state::WITHDRAW_DELAY;
use everlend_liquidity_oracle::state::DistributionArray;
use everlend_registry::state::{Registry, RegistryMarkets};
use everlend_utils::integrations;
use everlend_utils::integrations::MoneyMarketPubkeys;
use solana_account_decoder::parse_token::UiTokenAmount;
use solana_clap_utils::input_parsers::value_of;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;

const ARG_CASE: &str = "case";

pub struct TestCommand;

impl<'a> ToolkitCommand<'a> for TestCommand {
    fn get_name(&self) -> &'a str {
        "test"
    }

    fn get_description(&self) -> &'a str {
        "Run a test"
    }

    fn get_args(&self) -> Vec<Arg<'a, 'a>> {
        vec![arg(ARG_CASE, true).value_name("NAME").index(1).help("Case")]
    }

    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>> {
        vec![]
    }

    fn handle(&self, config: &Config, arg_matches: Option<&ArgMatches>) -> anyhow::Result<()> {
        let arg_matches = arg_matches.unwrap();
        let case = value_of::<String>(arg_matches, ARG_CASE);

        println!("Run {:?}", case);

        let default_accounts = config.get_default_accounts();
        let initialized_accounts = config.get_initialized_accounts();
        println!("default_accounts = {:#?}", default_accounts);

        let InitializedAccounts {
            payer: _,
            registry,
            general_pool_market,
            income_pool_market,
            mm_pool_markets: _,
            collateral_pool_markets,
            token_accounts,
            liquidity_oracle,
            depositor,
            quarry_mining: _,
            rebalance_executor: _,
            rewards_root: _,
        } = initialized_accounts;

        let registry_account = config.rpc_client.get_account(&registry)?;
        let registry_config = Registry::unpack(&registry_account.data).unwrap();
        let registry_markets = RegistryMarkets::unpack_from_slice(&registry_account.data).unwrap();

        println!("registry_config = {:#?}", registry_config);
        println!("registry_markets = {:#?}", registry_markets);

        let sol = token_accounts.get("SOL").unwrap();

        let sol_oracle = default_accounts.sol_oracle;
        let port_finance_pubkeys = integrations::spl_token_lending::AccountPubkeys {
            reserve: default_accounts.port_finance.reserve_sol,
            reserve_liquidity_supply: default_accounts.port_finance.reserve_sol_supply,
            reserve_liquidity_oracle: sol_oracle,
            lending_market: default_accounts.port_finance.lending_market,
        };
        let larix_pubkeys = integrations::larix::AccountPubkeys {
            reserve: default_accounts.larix.reserve_sol,
            reserve_liquidity_supply: default_accounts.larix.reserve_sol_supply,
            reserve_liquidity_oracle: sol_oracle,
            lending_market: default_accounts.larix.lending_market,
        };

        let solend_pubkeys = integrations::solend::AccountPubkeys {
            reserve: default_accounts.solend.reserve_sol,
            reserve_liquidity_supply: default_accounts
                .solend
                .reserve_sol_supply
                .context("`solend_reserve_sol_supply` invalid value")
                .unwrap(),
            reserve_liquidity_pyth_oracle: default_accounts
                .solend
                .reserve_pyth_oracle
                .context("`solend_reserve_pyth_oracle` invalid value")
                .unwrap(),
            reserve_liquidity_switchboard_oracle: default_accounts
                .solend
                .reserve_switchboard_oracle
                .context("`solend_reserve_switchboard_oracle` invalid value")
                .unwrap(),
            lending_market: default_accounts.solend.lending_market,
        };

        let tulip_pubkeys = integrations::tulip::AccountPubkeys {
            lending_market: default_accounts.tulip.lending_market,
            reserve_liquidity_oracle: default_accounts.tulip.reserve_liquidity_oracle,
            reserve: default_accounts.tulip.reserve_sol,
            reserve_liquidity_supply: default_accounts.tulip.reserve_liquidity_supply,
        };

        let get_balance = |pk: &Pubkey| config.rpc_client.get_token_account_balance(pk);

        let print_balance = |v: (UiTokenAmount, UiTokenAmount)| {
            println!(
                "Balance:\n\
             - liquidity_transit: {}\n\
             - general_pool_token_account: {}",
                v.0.amount, v.1.amount
            );
        };

        let update_liquidity_distribution = |d: DistributionArray| {
            update_liquidity_distribution(config, &liquidity_oracle, &sol.mint, &d)
        };

        let withdraw_requests =
            || get_withdrawal_request_accounts(config, &general_pool_market, &sol.mint);

        let start_rebalancing = || {
            println!("Rebalancing");
            start_rebalancing(
                config,
                &registry,
                &depositor,
                &sol.mint,
                &general_pool_market,
                &sol.general_pool_token_account,
                &liquidity_oracle,
                false,
            )
        };

        let refresh_income = || {
            println!("Rebalancing (Refresh income)");
            helper_start_rebalancing(
                config,
                &registry,
                &depositor,
                &sol.mint,
                &general_pool_market,
                &sol.general_pool_token_account,
                &liquidity_oracle,
                true,
            )
        };

        let deposit = |i: usize| {
            println!("Rebalancing: Deposit: {}", i);
            let pubkeys = match i {
                0 => MoneyMarketPubkeys::SPL(port_finance_pubkeys.clone()),
                1 => MoneyMarketPubkeys::Larix(larix_pubkeys.clone()),
                2 => MoneyMarketPubkeys::Solend(solend_pubkeys.clone()),
                3 => MoneyMarketPubkeys::Tulip(tulip_pubkeys.clone()),
                _ => panic!("wrong pubkey idx"),
            };

            depositor_deposit(
                config,
                &registry,
                &depositor,
                &sol.mint,
                &sol.collateral_pools[i].token_mint,
                &registry_markets.money_markets[i],
                integrations::deposit_accounts(&registry_markets.money_markets[i], &pubkeys),
                everlend_depositor::utils::collateral_pool_deposit_accounts(
                    &collateral_pool_markets[i],
                    &sol.collateral_pools[i].token_mint,
                    &sol.collateral_pools[i].pool_token_account,
                ),
            )
        };

        let withdraw = |i| {
            println!("Rebalancing: Withdraw: {}", i);
            let pubkeys = match i {
                0 => MoneyMarketPubkeys::SPL(port_finance_pubkeys.clone()),
                1 => MoneyMarketPubkeys::Larix(larix_pubkeys.clone()),
                2 => MoneyMarketPubkeys::Solend(solend_pubkeys.clone()),
                3 => MoneyMarketPubkeys::Tulip(tulip_pubkeys.clone()),
                _ => panic!("wrong pubkey idx"),
            };

            depositor_withdraw(
                config,
                &registry,
                &depositor,
                &income_pool_market,
                &sol.income_pool_token_account,
                &sol.collateral_pools[i].token_mint,
                &sol.mint,
                &registry_markets.money_markets[i],
                integrations::withdraw_accounts(&registry_markets.money_markets[i], &pubkeys),
                everlend_depositor::utils::collateral_pool_withdraw_accounts(
                    &collateral_pool_markets[i],
                    &sol.collateral_pools[i].token_mint,
                    &sol.collateral_pools[i].pool_token_account,
                    &everlend_depositor::id(),
                    &depositor,
                ),
            )
        };

        let complete_rebalancing = |rebalancing: Option<Rebalancing>| -> anyhow::Result<()> {
            let rebalancing = rebalancing.or_else(|| {
                let (rebalancing_pubkey, _) = find_rebalancing_program_address(
                    &everlend_depositor::id(),
                    &depositor,
                    &sol.mint,
                );
                config
                    .rpc_client
                    .get_account(&rebalancing_pubkey)
                    .ok()
                    .and_then(|a| Rebalancing::unpack(&a.data).ok())
            });

            if rebalancing.is_none() {
                return Ok(());
            }

            let rebalancing = rebalancing.unwrap();
            println!("{:#?}", rebalancing);
            print_balance((
                get_balance(&sol.liquidity_transit)?,
                get_balance(&sol.general_pool_token_account)?,
            ));

            for step in rebalancing
                .steps
                .iter()
                .filter(|&step| step.executed_at.is_none())
            {
                match step.operation {
                    RebalancingOperation::Deposit => deposit(step.money_market_index.into())?,
                    RebalancingOperation::Withdraw => withdraw(step.money_market_index.into())?,
                    RebalancingOperation::RefreshWithdraw => todo!(),
                    RebalancingOperation::RefreshDeposit => todo!(),
                }
            }

            print_balance((
                get_balance(&sol.liquidity_transit)?,
                get_balance(&sol.general_pool_token_account)?,
            ));

            Ok(())
        };

        let general_pool_deposit = |a: u64| {
            println!("Deposit liquidity");
            helper_deposit(
                config,
                &general_pool_market,
                &sol.general_pool,
                &sol.liquidity_token_account,
                &sol.collateral_token_account,
                &sol.general_pool_token_account,
                &sol.general_pool_mint,
                // TODO fix mocks
                &Pubkey::new_unique(),
                &Pubkey::new_unique(),
                a,
            )
        };

        let general_pool_withdraw_request = |a: u64| {
            println!("Withdraw request");
            withdraw_request(
                config,
                &general_pool_market,
                &sol.general_pool,
                &sol.collateral_token_account,
                &sol.liquidity_token_account,
                &sol.general_pool_token_account,
                &sol.mint,
                &sol.general_pool_mint,
                // TODO fix mocks
                &Pubkey::new_unique(),
                &Pubkey::new_unique(),
                a,
            )
        };

        let general_pool_withdraw = || {
            println!("Withdraw");
            helper_withdraw(
                config,
                &general_pool_market,
                &sol.general_pool,
                &sol.liquidity_token_account,
                &sol.general_pool_token_account,
                &sol.mint,
                &sol.general_pool_mint,
            )
        };

        complete_rebalancing(None)?;

        match case.as_deref() {
            Some("first") => {
                general_pool_deposit(1000)?;

                update_liquidity_distribution(distribution!([1000000000, 0]))?;
                let (_, rebalancing) = start_rebalancing()?;
                complete_rebalancing(Some(rebalancing))?;

                update_liquidity_distribution(distribution!([959876767, 0]))?;
                let (_, rebalancing) = start_rebalancing()?;
                complete_rebalancing(Some(rebalancing))?;

                update_liquidity_distribution(distribution!([959876767, 0]))?;
                let (_, rebalancing) = start_rebalancing()?;
                complete_rebalancing(Some(rebalancing))?;

                update_liquidity_distribution(distribution!([959876767, 0]))?;
                let (_, rebalancing) = start_rebalancing()?;
                println!("{:#?}", rebalancing);
            }
            Some("second") => {
                general_pool_deposit(1000)?;

                update_liquidity_distribution(distribution!([500000000, 500000000]))?;
                let (_, rebalancing) = start_rebalancing()?;
                complete_rebalancing(Some(rebalancing))?;

                general_pool_deposit(10)?;

                update_liquidity_distribution(distribution!([900000000, 100000000]))?;
                let (_, rebalancing) = start_rebalancing()?;
                complete_rebalancing(Some(rebalancing))?;
            }
            Some("third") => {
                general_pool_deposit(1000)?;

                update_liquidity_distribution(distribution!([100000000, 100000000, 800000000]))?;
                let (_, rebalancing) = start_rebalancing()?;
                complete_rebalancing(Some(rebalancing))?;

                general_pool_deposit(10)?;

                update_liquidity_distribution(distribution!([0, 300000000, 700000000]))?;
                let (_, rebalancing) = start_rebalancing()?;
                complete_rebalancing(Some(rebalancing))?;
            }
            Some("invalid-amount") => {
                general_pool_deposit(1000)?;

                update_liquidity_distribution(distribution!([1000000000, 0, 0]))?;
                let (_, rebalancing) = start_rebalancing()?;
                complete_rebalancing(Some(rebalancing))?;

                general_pool_withdraw_request(
                    get_balance(&sol.general_pool_token_account)?
                        .amount
                        .parse::<u64>()
                        .unwrap(),
                )?;
                let (_, rebalancing) = refresh_income()?;
                complete_rebalancing(Some(rebalancing))?;
            }
            Some("larix") => {
                general_pool_deposit(1000)?;

                update_liquidity_distribution(distribution!([0, 0, 1000000000]))?;
                let (_, rebalancing) = start_rebalancing()?;
                complete_rebalancing(Some(rebalancing))?;
            }
            Some("solend") => {
                general_pool_deposit(1000)?;

                update_liquidity_distribution(distribution!([0, 0, 1000000000]))?;
                let (_, rebalancing) = start_rebalancing()?;
                complete_rebalancing(Some(rebalancing))?;
            }
            Some("zero-distribution") => {
                general_pool_deposit(1000)?;

                update_liquidity_distribution(distribution!([0, 0]))?;
                let (_, rebalancing) = start_rebalancing()?;
                complete_rebalancing(Some(rebalancing))?;
            }
            Some("deposit") => {
                general_pool_deposit(1000)?;

                update_liquidity_distribution(distribution!([1000000000, 0]))?;
                let (_, rebalancing) = start_rebalancing()?;
                complete_rebalancing(Some(rebalancing))?;

                general_pool_deposit(1000)?;

                update_liquidity_distribution(distribution!([1000000000, 0]))?;
                let (_, rebalancing) = start_rebalancing()?;
                println!("{:#?}", rebalancing);
            }
            Some("full") => {
                general_pool_deposit(1000)?;

                update_liquidity_distribution(distribution!([1000000000, 0]))?;
                let (_, rebalancing) = start_rebalancing()?;
                complete_rebalancing(Some(rebalancing))?;

                general_pool_withdraw_request(100)?;
                let withdraw_requests = withdraw_requests()?;
                println!("{:#?}", withdraw_requests);

                update_liquidity_distribution(distribution!([1000000000, 0]))?;
                let (_, rebalancing) = start_rebalancing()?;
                complete_rebalancing(Some(rebalancing))?;

                update_liquidity_distribution(distribution!([1000000000, 0]))?;
                let (_, rebalancing) = start_rebalancing()?;
                println!("{:#?}", rebalancing);

                delay(WITHDRAW_DELAY / 2);
                general_pool_withdraw()?;

                update_liquidity_distribution(distribution!([1000000000, 0]))?;
                let (_, rebalancing) = start_rebalancing()?;
                println!("{:#?}", rebalancing);
            }
            Some("withdraw") => {
                general_pool_withdraw()?;
            }
            Some("11") => {
                general_pool_deposit(4321)?;

                update_liquidity_distribution(distribution!([10, 10]))?;
                let (_, rebalancing) = start_rebalancing()?;
                complete_rebalancing(Some(rebalancing))?;

                update_liquidity_distribution(distribution!([10, 20]))?;
                let (_, rebalancing) = start_rebalancing()?;
                complete_rebalancing(Some(rebalancing))?;
            }
            Some("empty") => {
                update_liquidity_distribution(distribution!([1000000000, 0]))?;
                start_rebalancing()?;
            }
            Some("refresh-income") => {
                general_pool_deposit(1000)?;

                update_liquidity_distribution(distribution!([1000000000, 0]))?;
                let (_, rebalancing) = start_rebalancing()?;
                complete_rebalancing(Some(rebalancing))?;

                let (_, rebalancing) = refresh_income()?;
                println!("{:#?}", rebalancing);
            }
            None => {
                general_pool_deposit(1000)?;

                update_liquidity_distribution(distribution!([500000000, 500000000]))?;
                let (_, rebalancing) = start_rebalancing()?;
                complete_rebalancing(Some(rebalancing))?;

                general_pool_withdraw_request(100)?;

                update_liquidity_distribution(distribution!([300000000, 600000000]))?;
                let (_, rebalancing) = start_rebalancing()?;
                complete_rebalancing(Some(rebalancing))?;

                update_liquidity_distribution(distribution!([0, 1000000000]))?;
                let (_, rebalancing) = start_rebalancing()?;
                delay(WITHDRAW_DELAY / 2);
                general_pool_withdraw()?;
                complete_rebalancing(Some(rebalancing))?;

                update_liquidity_distribution(distribution!([100000000, 0]))?;
                let (_, rebalancing) = start_rebalancing()?;
                complete_rebalancing(Some(rebalancing))?;
            }
            _ => {}
        }

        println!("Finished!");

        Ok(())
    }
}
