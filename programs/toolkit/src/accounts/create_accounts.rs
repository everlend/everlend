use crate::accounts_config::CollateralPoolAccounts;
use crate::accounts_config::TokenAccounts;
use crate::helpers::{
    create_collateral_market, create_collateral_pool, create_general_pool,
    create_general_pool_market, create_income_pool, create_income_pool_market,
    create_pool_borrow_authority, create_pool_withdraw_authority, create_token_distribution,
    create_transit, init_depositor, init_liquidity_oracle, init_registry, set_registry_config,
    PoolPubkeys,
};
use crate::utils::{
    arg_multiple, arg_pubkey, get_asset_maps, spl_create_associated_token_account,
    spl_token_transfer, REFRESH_INCOME_INTERVAL,
};
use crate::{Config, InitializedAccounts, ToolkitCommand, ARG_ACCOUNTS};
use clap::{Arg, ArgMatches};
use everlend_liquidity_oracle::state::DistributionArray;
use everlend_registry::state::{
    RegistryPrograms, RegistryRootAccounts, RegistrySettings, TOTAL_DISTRIBUTIONS,
};
use solana_clap_utils::input_parsers::pubkey_of;
use solana_client::client_error::ClientError;
use solana_program::pubkey::Pubkey;
use spl_associated_token_account::get_associated_token_address;
use std::collections::BTreeMap;

const ARG_MINTS: &str = "mints";
const ARG_REBALANCE_EXECUTOR: &str = "rebalance-executor";

#[derive(Clone, Copy)]
pub struct CreateAccountsCommand;

impl<'a> ToolkitCommand<'a> for CreateAccountsCommand {
    fn get_name(&self) -> &'a str {
        return "create";
    }

    fn get_description(&self) -> &'a str {
        return "Create a new accounts";
    }

    fn get_args(&self) -> Vec<Arg<'a, 'a>> {
        return vec![
            arg_multiple(ARG_MINTS, true).short("m"),
            arg_pubkey(ARG_REBALANCE_EXECUTOR, true).help("Rebalance executor pubkey"),
        ];
    }

    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>> {
        return vec![];
    }

    fn handle(&self, config: &Config, arg_matches: Option<&ArgMatches>) -> anyhow::Result<()> {
        let arg_matches = arg_matches.unwrap();

        let required_mints: Vec<_> = arg_matches.values_of(ARG_MINTS).unwrap().collect();
        let accounts_path = arg_matches.value_of(ARG_ACCOUNTS).unwrap();
        let rebalance_executor = pubkey_of(arg_matches, ARG_REBALANCE_EXECUTOR).unwrap();

        let payer_pubkey = config.fee_payer.pubkey();
        println!("Fee payer: {}", payer_pubkey);

        let default_accounts = config.get_default_accounts();

        let (mint_map, collateral_mint_map) = get_asset_maps(default_accounts.clone());

        println!("Registry");
        let registry_pubkey = init_registry(config, None)?;
        let mut programs = RegistryPrograms {
            general_pool_program_id: everlend_general_pool::id(),
            collateral_pool_program_id: everlend_collateral_pool::id(),
            liquidity_oracle_program_id: everlend_liquidity_oracle::id(),
            depositor_program_id: everlend_depositor::id(),
            income_pools_program_id: everlend_income_pools::id(),
            money_market_program_ids: [Pubkey::default(); TOTAL_DISTRIBUTIONS],
        };
        programs.money_market_program_ids[0] = default_accounts.port_finance.program_id;
        programs.money_market_program_ids[1] = default_accounts.larix.program_id;
        programs.money_market_program_ids[2] = default_accounts.solend.program_id;
        programs.money_market_program_ids[3] = default_accounts.tulip.program_id;

        set_registry_config(
            config,
            &registry_pubkey,
            programs,
            RegistryRootAccounts::default(),
            RegistrySettings {
                refresh_income_interval: REFRESH_INCOME_INTERVAL,
            },
        )?;
        println!("programs = {:#?}", programs);

        let general_pool_market_pubkey =
            create_general_pool_market(config, None, &registry_pubkey)?;
        let income_pool_market_pubkey =
            create_income_pool_market(config, None, &general_pool_market_pubkey)?;

        let mm_collateral_pool_markets = vec![
            create_collateral_market(config, None)?,
            create_collateral_market(config, None)?,
            create_collateral_market(config, None)?,
        ];

        println!("Liquidity oracle");
        let liquidity_oracle_pubkey = init_liquidity_oracle(config, None)?;
        let mut distribution = DistributionArray::default();
        distribution[0] = 0;
        distribution[1] = 0;
        distribution[2] = 0;

        println!("Registry");
        let registry_pubkey = init_registry(config, None)?;
        let mut programs = RegistryPrograms {
            general_pool_program_id: everlend_general_pool::id(),
            collateral_pool_program_id: everlend_collateral_pool::id(),
            liquidity_oracle_program_id: everlend_liquidity_oracle::id(),
            depositor_program_id: everlend_depositor::id(),
            income_pools_program_id: everlend_income_pools::id(),
            money_market_program_ids: [Pubkey::default(); TOTAL_DISTRIBUTIONS],
        };
        programs.money_market_program_ids[0] = default_accounts.port_finance.program_id;
        programs.money_market_program_ids[1] = default_accounts.larix.program_id;
        programs.money_market_program_ids[2] = default_accounts.solend.program_id;
        programs.money_market_program_ids[3] = default_accounts.tulip.program_id;

        println!("programs = {:#?}", programs);

        let mut collateral_pool_markets: [Pubkey; TOTAL_DISTRIBUTIONS] = Default::default();
        collateral_pool_markets[..mm_collateral_pool_markets.len()]
            .copy_from_slice(&mm_collateral_pool_markets);

        let roots = RegistryRootAccounts {
            general_pool_market: general_pool_market_pubkey,
            income_pool_market: income_pool_market_pubkey,
            collateral_pool_markets,
            liquidity_oracle: liquidity_oracle_pubkey,
        };

        println!("roots = {:#?}", &roots);

        set_registry_config(
            config,
            &registry_pubkey,
            programs,
            roots,
            RegistrySettings {
                refresh_income_interval: 0,
            },
        )?;

        println!("Depositor");
        let depositor_pubkey = init_depositor(config, &registry_pubkey, None, rebalance_executor)?;

        println!("Prepare borrow authority");
        let (depositor_authority, _) =
            &everlend_utils::find_program_address(&everlend_depositor::id(), &depositor_pubkey);

        let mut token_accounts = BTreeMap::new();

        for key in required_mints {
            let mint = mint_map.get(key).unwrap();
            let collateral_mints: Vec<(Pubkey, Pubkey)> = collateral_mint_map
                .get(key)
                .unwrap()
                .iter()
                .zip(mm_collateral_pool_markets.iter())
                .filter_map(|(collateral_mint, mm_pool_market_pubkey)| {
                    collateral_mint.map(|coll_mint| (coll_mint, *mm_pool_market_pubkey))
                })
                .collect();

            let (general_pool_pubkey, general_pool_token_account, general_pool_mint) =
                create_general_pool(config, &general_pool_market_pubkey, mint)?;

            let token_account = get_associated_token_address(&payer_pubkey, mint);
            let pool_account =
                spl_create_associated_token_account(config, &payer_pubkey, &general_pool_mint)?;

            let (income_pool_pubkey, income_pool_token_account) =
                create_income_pool(config, &income_pool_market_pubkey, mint)?;

            // MM Pools
            let mm_pool_collection = collateral_mints
                .iter()
                .map(
                    |(collateral_mint, mm_pool_market_pubkey)| -> Result<PoolPubkeys, ClientError> {
                        println!("MM Pool: {}", collateral_mint);
                        let pool_pubkeys =
                            create_collateral_pool(config, mm_pool_market_pubkey, collateral_mint)?;
                        create_pool_withdraw_authority(
                            config,
                            mm_pool_market_pubkey,
                            &pool_pubkeys.pool,
                            depositor_authority,
                            &config.fee_payer.pubkey(),
                        )?;

                        Ok(pool_pubkeys)
                    },
                )
                .collect::<Result<Vec<PoolPubkeys>, ClientError>>()?;

            create_token_distribution(config, &liquidity_oracle_pubkey, mint, &distribution)?;

            // Transit accounts
            let liquidity_transit_pubkey = create_transit(config, &depositor_pubkey, mint, None)?;

            // Reserve
            println!("Reserve transit");
            let liquidity_reserve_transit_pubkey =
                create_transit(config, &depositor_pubkey, mint, Some("reserve".to_string()))?;
            spl_token_transfer(
                config,
                &token_account,
                &liquidity_reserve_transit_pubkey,
                10000,
            )?;

            collateral_mints
                .iter()
                .map(|(collateral_mint, _mm_pool_market_pubkey)| {
                    create_transit(config, &depositor_pubkey, collateral_mint, None)
                })
                .collect::<Result<Vec<Pubkey>, ClientError>>()?;

            let collateral_pools = collateral_mints
                .iter()
                .zip(mm_pool_collection)
                .map(
                    |((collateral_mint, _mm_pool_market_pubkey), mm_pool_pubkeys)| {
                        CollateralPoolAccounts {
                            pool: mm_pool_pubkeys.pool,
                            pool_token_account: mm_pool_pubkeys.token_account,
                            token_mint: *collateral_mint,
                        }
                    },
                )
                .collect();

            // Borrow authorities
            create_pool_borrow_authority(
                config,
                &general_pool_market_pubkey,
                &general_pool_pubkey,
                depositor_authority,
                10_000, // 100%
            )?;

            token_accounts.insert(
                key.to_string(),
                TokenAccounts {
                    mint: *mint,
                    liquidity_token_account: token_account,
                    collateral_token_account: pool_account,
                    general_pool: general_pool_pubkey,
                    general_pool_token_account,
                    general_pool_mint,
                    income_pool: income_pool_pubkey,
                    income_pool_token_account,
                    mm_pools: Vec::new(),
                    mining_accounts: Vec::new(),
                    collateral_pools,
                    liquidity_transit: liquidity_transit_pubkey,
                    port_finance_obligation_account: Pubkey::default(),
                },
            );
        }

        let initialized_accounts = InitializedAccounts {
            payer: payer_pubkey,
            registry: registry_pubkey,
            general_pool_market: general_pool_market_pubkey,
            income_pool_market: income_pool_market_pubkey,
            mm_pool_markets: Vec::new(),
            collateral_pool_markets: mm_collateral_pool_markets,
            token_accounts,
            liquidity_oracle: liquidity_oracle_pubkey,
            depositor: depositor_pubkey,
            quarry_mining: BTreeMap::new(),
            rebalance_executor,
        };

        initialized_accounts.save(accounts_path).unwrap();

        Ok(())
    }
}
