use crate::accounts_config::CollateralPoolAccounts;
use crate::accounts_config::TokenAccounts;
use crate::helpers::{
    create_collateral_market, create_collateral_pool, create_general_pool,
    create_general_pool_market, create_income_pool, create_income_pool_market,
    create_pool_borrow_authority, create_pool_withdraw_authority, create_token_oracle,
    create_transit, init_depositor, init_liquidity_oracle, init_registry, init_rewards_root,
    update_registry, update_registry_markets, PoolPubkeys,
};
use crate::utils::{
    arg_multiple, arg_pubkey, get_asset_maps, spl_create_associated_token_account,
    spl_token_transfer, REFRESH_INCOME_INTERVAL,
};
use crate::{arg_keypair, Config, InitializedAccounts, ToolkitCommand, ARG_ACCOUNTS};
use clap::{Arg, ArgMatches};
use everlend_liquidity_oracle::state::DistributionArray;
use everlend_registry::instructions::{UpdateRegistryData, UpdateRegistryMarketsData};
use everlend_registry::state::DistributionPubkeys;
use solana_clap_utils::input_parsers::{keypair_of, pubkey_of};
use solana_client::client_error::ClientError;
use solana_program::pubkey::Pubkey;
use spl_associated_token_account::get_associated_token_address;
use std::collections::BTreeMap;

const ARG_MINTS: &str = "mints";
const ARG_REBALANCE_EXECUTOR: &str = "rebalance-executor";
const ARG_REWARDS_ROOT: &str = "rewards-root";

#[derive(Clone, Copy)]
pub struct CreateAccountsCommand;

impl<'a> ToolkitCommand<'a> for CreateAccountsCommand {
    fn get_name(&self) -> &'a str {
        "create"
    }

    fn get_description(&self) -> &'a str {
        "Create a new accounts"
    }

    fn get_args(&self) -> Vec<Arg<'a, 'a>> {
        vec![
            arg_multiple(ARG_MINTS, true).short("m"),
            arg_pubkey(ARG_REBALANCE_EXECUTOR, true).help("Rebalance executor pubkey"),
            arg_keypair(ARG_REWARDS_ROOT, true).help("Rewards root keypair"),
        ]
    }

    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>> {
        vec![]
    }

    fn handle(&self, config: &Config, arg_matches: Option<&ArgMatches>) -> anyhow::Result<()> {
        let arg_matches = arg_matches.unwrap();

        let required_mints: Vec<_> = arg_matches.values_of(ARG_MINTS).unwrap().collect();
        let accounts_path = arg_matches.value_of(ARG_ACCOUNTS).unwrap();
        let rebalance_executor = pubkey_of(arg_matches, ARG_REBALANCE_EXECUTOR).unwrap();
        let rewards_root_keypair = keypair_of(arg_matches, ARG_REWARDS_ROOT).unwrap();

        let payer_pubkey = config.fee_payer.pubkey();
        println!("Fee payer: {}", payer_pubkey);

        let default_accounts = config.get_default_accounts();

        let (mint_map, collateral_mint_map) = get_asset_maps(default_accounts.clone());

        println!("Registry");
        let registry_pubkey = init_registry(config, None)?;

        let general_pool_market_pubkey =
            create_general_pool_market(config, None, &registry_pubkey)?;
        let income_pool_market_pubkey =
            create_income_pool_market(config, None, &general_pool_market_pubkey)?;

        println!("Liquidity oracle");
        let liquidity_oracle_pubkey = init_liquidity_oracle(config, None)?;
        let mut distribution = DistributionArray::default();
        distribution[0] = 0;
        distribution[1] = 0;
        distribution[2] = 0;
        distribution[3] = 0;
        distribution[4] = 0;

        println!("Registry");
        let mut money_market_program_ids = DistributionPubkeys::default();
        money_market_program_ids[0] = default_accounts.port_finance.program_id;
        money_market_program_ids[1] = default_accounts.larix.program_id;
        money_market_program_ids[2] = default_accounts.solend.program_id;
        money_market_program_ids[3] = default_accounts.tulip.program_id;
        money_market_program_ids[4] = default_accounts.francium.program_id;

        let mm_collateral_pool_markets = vec![
            create_collateral_market(config, None)?,
            create_collateral_market(config, None)?,
            create_collateral_market(config, None)?,
            create_collateral_market(config, None)?,
            create_collateral_market(config, None)?,
        ];

        let mut collateral_pool_markets = DistributionPubkeys::default();
        collateral_pool_markets[..mm_collateral_pool_markets.len()]
            .copy_from_slice(&mm_collateral_pool_markets);

        update_registry(
            config,
            &registry_pubkey,
            UpdateRegistryData {
                general_pool_market: Some(general_pool_market_pubkey),
                income_pool_market: Some(income_pool_market_pubkey),
                liquidity_oracle: Some(liquidity_oracle_pubkey),
                refresh_income_interval: Some(REFRESH_INCOME_INTERVAL),
            },
        )?;

        update_registry_markets(
            config,
            &registry_pubkey,
            UpdateRegistryMarketsData {
                money_markets: Some(money_market_program_ids),
                collateral_pool_markets: Some(collateral_pool_markets),
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

            create_token_oracle(config, &liquidity_oracle_pubkey, mint, &distribution)?;

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

        let rewards_root = init_rewards_root(config, rewards_root_keypair)?;

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
            rewards_root,
        };

        initialized_accounts.save(accounts_path).unwrap();

        Ok(())
    }
}
