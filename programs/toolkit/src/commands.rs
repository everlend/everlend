use std::fs;

use crate::accounts_config::{save_config_file, CollateralPoolAccounts, DefaultAccounts};
use crate::collateral_pool::{self, PoolPubkeys};
use crate::download_account::download_account;
use crate::liquidity_mining::quarry_liquidity_miner::QuarryLiquidityMiner;
use crate::liquidity_mining::save_mining_accounts;
use crate::liquidity_mining::{
    execute_init_mining_accounts, larix_liquidity_miner::LarixLiquidityMiner,
    port_liquidity_miner::PortLiquidityMiner, quarry_raw_test, LiquidityMiner,
};
use crate::registry::close_registry_config;
use crate::utils::init_token_account;
use crate::{
    accounts_config::TokenAccounts,
    depositor, general_pool, income_pools, liquidity_oracle, registry,
    utils::{
        get_asset_maps, spl_create_associated_token_account, spl_token_transfer, Config,
        REFRESH_INCOME_INTERVAL,
    },
};
use anchor_lang::AnchorDeserialize;
use anyhow::bail;
use everlend_depositor::state::Rebalancing;
use everlend_general_pool::state::SetPoolConfigParams;
use everlend_liquidity_oracle::state::DistributionArray;
use everlend_registry::state::{DeprecatedRegistryConfig, Registry};
use everlend_registry::{
    find_config_program_address,
    state::{
        RegistryConfig, RegistryPrograms, RegistryRootAccounts, RegistrySettings,
        TOTAL_DISTRIBUTIONS,
    },
};
use everlend_utils::integrations::{MoneyMarket, StakingMoneyMarket};
use larix_lending::state::reserve::Reserve;
use solana_client::client_error::ClientError;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;
use solana_program_test::{find_file, read_file};
use solana_sdk::signature::Keypair;
use solana_sdk::signature::Signer;
use spl_associated_token_account::get_associated_token_address;

pub fn command_create_registry(config: &Config, keypair: Option<Keypair>) -> anyhow::Result<()> {
    let payer_pubkey = config.fee_payer.pubkey();
    println!("Fee payer: {}", payer_pubkey);

    let default_accounts = config.get_default_accounts();
    let mut initialized_accounts = config.get_initialized_accounts();

    let mm_pool_markets = &initialized_accounts.mm_pool_markets;

    let registry_pubkey = registry::init(config, keypair)?;
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
    collateral_pool_markets[..mm_pool_markets.len()].copy_from_slice(mm_pool_markets);

    let roots = RegistryRootAccounts {
        general_pool_market: initialized_accounts.general_pool_market,
        income_pool_market: initialized_accounts.income_pool_market,
        collateral_pool_markets,
        liquidity_oracle: initialized_accounts.liquidity_oracle,
    };

    println!("roots = {:#?}", &roots);

    registry::set_registry_config(
        config,
        &registry_pubkey,
        programs,
        roots,
        RegistrySettings {
            refresh_income_interval: REFRESH_INCOME_INTERVAL,
        },
    )?;

    initialized_accounts.payer = payer_pubkey;
    initialized_accounts.registry = registry_pubkey;

    initialized_accounts
        .save(&format!("accounts.{}.yaml", config.network))
        .unwrap();

    Ok(())
}

pub fn command_init_quarry_mining_accounts(config: &Config, token: &String) -> anyhow::Result<()> {
    let default_accounts = config.get_default_accounts();
    let mut initialized_accounts = config.get_initialized_accounts();
    let quarry_mining = initialized_accounts.quarry_mining.get_mut(token).unwrap();
    let miner_vault = Keypair::new();
    quarry_raw_test::create_miner(config, &miner_vault)?;
    quarry_mining.miner_vault = miner_vault.pubkey();
    println!("miner vault {}", miner_vault.pubkey());
    let token_source = Keypair::new();
    init_token_account(config, &token_source, &default_accounts.quarry.token_mint)?;
    quarry_mining.token_source = token_source.pubkey();
    println!("token source {}", token_source.pubkey());
    let rewards_account = Keypair::new();
    init_token_account(
        config,
        &rewards_account,
        &default_accounts.quarry.rewards_token_mint,
    )?;
    quarry_mining.rewards_token_account = rewards_account.pubkey();
    println!("rewards token account {}", rewards_account.pubkey());
    let fee_account = Keypair::new();
    init_token_account(
        config,
        &fee_account,
        &default_accounts.quarry.rewards_token_mint,
    )?;
    quarry_mining.fee_token_account = fee_account.pubkey();
    println!("fee token account {}", fee_account.pubkey());
    initialized_accounts.save(&format!("accounts.{}.yaml", config.network))?;
    Ok(())
}

pub async fn command_set_pool_config(
    config: &Config,
    pool_pubkey: Pubkey,
    params: SetPoolConfigParams,
) -> anyhow::Result<()> {
    let initialized_accounts = config.get_initialized_accounts();
    general_pool::set_pool_config(
        config,
        &initialized_accounts.general_pool_market,
        &pool_pubkey,
        params,
    )?;

    Ok(())
}

pub async fn command_create_general_pool_market(
    config: &Config,
    keypair: Option<Keypair>,
    registry: Pubkey,
) -> anyhow::Result<()> {
    let mut initialiazed_accounts = config.get_initialized_accounts();

    let general_pool_market_pubkey = general_pool::create_market(config, keypair, &registry)?;

    initialiazed_accounts.general_pool_market = general_pool_market_pubkey;

    initialiazed_accounts
        .save(&format!("accounts.{}.yaml", config.network))
        .unwrap();

    Ok(())
}

pub async fn command_create_income_pool_market(
    config: &Config,
    keypair: Option<Keypair>,
) -> anyhow::Result<()> {
    let mut initialiazed_accounts = config.get_initialized_accounts();

    let income_pool_market_pubkey =
        income_pools::create_market(config, keypair, &initialiazed_accounts.general_pool_market)?;

    initialiazed_accounts.income_pool_market = income_pool_market_pubkey;

    initialiazed_accounts
        .save(&format!("accounts.{}.yaml", config.network))
        .unwrap();

    Ok(())
}

pub async fn command_create_collateral_pool_market(
    config: &Config,
    keypair: Option<Keypair>,
    money_market: MoneyMarket,
) -> anyhow::Result<()> {
    let mut initialiazed_accounts = config.get_initialized_accounts();

    let mm_pool_market_pubkey = collateral_pool::create_market(config, keypair)?;

    initialiazed_accounts.mm_pool_markets[money_market as usize] = mm_pool_market_pubkey;

    initialiazed_accounts
        .save(&format!("accounts.{}.yaml", config.network))
        .unwrap();

    Ok(())
}

pub async fn command_create_liquidity_oracle(
    config: &Config,
    keypair: Option<Keypair>,
) -> anyhow::Result<()> {
    let mut initialiazed_accounts = config.get_initialized_accounts();

    let liquidity_oracle_pubkey = liquidity_oracle::init(config, keypair)?;

    initialiazed_accounts.liquidity_oracle = liquidity_oracle_pubkey;

    initialiazed_accounts
        .save(&format!("accounts.{}.yaml", config.network))
        .unwrap();

    Ok(())
}

pub async fn command_update_liquidity_oracle(
    config: &Config,
    authority: Keypair,
    new_authority: Keypair,
) -> anyhow::Result<()> {
    let initialiazed_accounts = config.get_initialized_accounts();

    println!(
        "oracle {} new authority {}",
        initialiazed_accounts.liquidity_oracle,
        new_authority.pubkey()
    );

    liquidity_oracle::update(
        config,
        initialiazed_accounts.liquidity_oracle,
        authority,
        new_authority,
    )?;

    Ok(())
}

pub async fn command_create_depositor(
    config: &Config,
    keypair: Option<Keypair>,
    rebalance_executor: Pubkey,
) -> anyhow::Result<()> {
    let mut initialiazed_accounts = config.get_initialized_accounts();

    let depositor_pubkey = depositor::init(
        config,
        &initialiazed_accounts.registry,
        keypair,
        rebalance_executor,
        // &initialiazed_accounts.general_pool_market,
        // &initialiazed_accounts.income_pool_market,
        // &initialiazed_accounts.liquidity_oracle,
    )?;

    initialiazed_accounts.depositor = depositor_pubkey;
    initialiazed_accounts.rebalance_executor = rebalance_executor;

    initialiazed_accounts
        .save(&format!("accounts.{}.yaml", config.network))
        .unwrap();

    Ok(())
}

#[allow(dead_code)]
pub async fn command_create_mm_pool(
    config: &Config,
    money_market: MoneyMarket,
    required_mints: Vec<&str>,
) -> anyhow::Result<()> {
    let default_accounts = config.get_default_accounts();
    let mut initialiazed_accounts = config.get_initialized_accounts();

    let (_, collateral_mint_map) = get_asset_maps(default_accounts);
    let money_market_index = money_market as usize;
    let mm_pool_market_pubkey = initialiazed_accounts.mm_pool_markets[money_market_index];

    for key in required_mints {
        let collateral_mint = collateral_mint_map.get(key).unwrap()[money_market_index].unwrap();

        let pool_pubkeys =
            collateral_pool::create_pool(config, &mm_pool_market_pubkey, &collateral_mint)?;

        depositor::create_transit(
            config,
            &initialiazed_accounts.depositor,
            &collateral_mint,
            None,
        )?;

        let money_market_accounts = CollateralPoolAccounts {
            pool: pool_pubkeys.pool,
            pool_token_account: pool_pubkeys.token_account,
            token_mint: collateral_mint,
        };

        initialiazed_accounts
            .token_accounts
            .get_mut(key)
            .unwrap()
            .collateral_pools[money_market_index] = money_market_accounts;
    }

    initialiazed_accounts
        .save(&format!("accounts.{}.yaml", config.network))
        .unwrap();

    Ok(())
}

pub async fn command_create_collateral_pool(
    config: &Config,
    money_market: MoneyMarket,
    required_mints: Vec<&str>,
) -> anyhow::Result<()> {
    let default_accounts = config.get_default_accounts();
    let mut initialiazed_accounts = config.get_initialized_accounts();

    let (_, collateral_mint_map) = get_asset_maps(default_accounts);
    let money_market_index = money_market as usize;
    let mm_pool_market_pubkey = initialiazed_accounts.mm_pool_markets[money_market_index];

    for key in required_mints {
        let collateral_mint = collateral_mint_map.get(key).unwrap()[money_market_index].unwrap();

        let pool_pubkeys =
            collateral_pool::create_pool(config, &mm_pool_market_pubkey, &collateral_mint)?;

        depositor::create_transit(
            config,
            &initialiazed_accounts.depositor,
            &collateral_mint,
            None,
        )?;

        let money_market_accounts = CollateralPoolAccounts {
            pool: pool_pubkeys.pool,
            pool_token_account: pool_pubkeys.token_account,
            token_mint: collateral_mint,
        };

        initialiazed_accounts
            .token_accounts
            .get_mut(key)
            .unwrap()
            .collateral_pools[money_market_index] = money_market_accounts;
    }

    initialiazed_accounts
        .save(&format!("accounts.{}.yaml", config.network))
        .unwrap();

    Ok(())
}

pub async fn command_create_token_accounts(
    config: &Config,
    required_mints: Vec<&str>,
) -> anyhow::Result<()> {
    let payer_pubkey = config.fee_payer.pubkey();
    let default_accounts = config.get_default_accounts();
    let mut initialiazed_accounts = config.get_initialized_accounts();

    let (mint_map, collateral_mint_map) = get_asset_maps(default_accounts);

    let mut distribution = DistributionArray::default();
    distribution[0] = 0;
    distribution[1] = 0;
    distribution[2] = 0;

    println!("Prepare borrow authority");
    let (depositor_authority, _) = &everlend_utils::find_program_address(
        &everlend_depositor::id(),
        &initialiazed_accounts.depositor,
    );

    for key in required_mints {
        let mint = mint_map.get(key).unwrap();
        let collateral_mints: Vec<(Pubkey, Pubkey)> = collateral_mint_map
            .get(key)
            .unwrap()
            .iter()
            .zip(initialiazed_accounts.mm_pool_markets.iter())
            .filter_map(|(collateral_mint, mm_pool_market_pubkey)| {
                collateral_mint.map(|coll_mint| (coll_mint, *mm_pool_market_pubkey))
            })
            .collect();

        println!("General pool");
        let (general_pool_pubkey, general_pool_token_account, general_pool_mint) =
            general_pool::create_pool(config, &initialiazed_accounts.general_pool_market, mint)?;

        let token_account = get_associated_token_address(&payer_pubkey, mint);
        println!("Payer token account: {:?}", token_account);
        // let pool_account = get_associated_token_address(&payer_pubkey, &general_pool_mint);
        let pool_account =
            spl_create_associated_token_account(config, &payer_pubkey, &general_pool_mint)
                .unwrap_or_else(|_| {
                    get_associated_token_address(&payer_pubkey, &general_pool_mint)
                });
        println!("Payer pool account: {:?}", pool_account);

        println!("Income pool");
        let (income_pool_pubkey, income_pool_token_account) =
            income_pools::create_pool(config, &initialiazed_accounts.income_pool_market, mint)?;

        // MM Pools
        let mm_pool_pubkeys = collateral_mints
            .iter()
            .map(|(collateral_mint, mm_pool_market_pubkey)| {
                println!("MM Pool: {}", collateral_mint);
                if collateral_mint.eq(&Pubkey::default()) {
                    // We can't skip cuz of mm pools is indexed
                    Ok(PoolPubkeys {
                        pool: Pubkey::default(),
                        token_account: Pubkey::default(),
                    })
                } else {
                    collateral_pool::create_pool(config, mm_pool_market_pubkey, collateral_mint)
                }
            })
            .collect::<Result<Vec<PoolPubkeys>, ClientError>>()?;

        liquidity_oracle::create_token_distribution(
            config,
            &initialiazed_accounts.liquidity_oracle,
            mint,
            &distribution,
        )?;

        // Transit accounts
        let liquidity_transit_pubkey =
            depositor::create_transit(config, &initialiazed_accounts.depositor, mint, None)?;

        // Reserve
        println!("Reserve transit");
        depositor::create_transit(
            config,
            &initialiazed_accounts.depositor,
            mint,
            Some("reserve".to_string()),
        )?;

        println!("Collateral transits");
        collateral_mints
            .iter()
            .filter(|(pk, _)| !pk.eq(&Pubkey::default()))
            .map(|(collateral_mint, _mm_pool_market_pubkey)| {
                depositor::create_transit(
                    config,
                    &initialiazed_accounts.depositor,
                    collateral_mint,
                    None,
                )
            })
            .collect::<Result<Vec<Pubkey>, ClientError>>()?;

        let collateral_pools = collateral_mints
            .iter()
            .zip(mm_pool_pubkeys)
            .map(
                |((collateral_mint, _mm_pool_market_pubkey), pubkeys)| CollateralPoolAccounts {
                    pool: pubkeys.pool,
                    pool_token_account: pubkeys.token_account,
                    token_mint: *collateral_mint,
                },
            )
            .collect();

        // Borrow authorities
        general_pool::create_pool_borrow_authority(
            config,
            &initialiazed_accounts.general_pool_market,
            &general_pool_pubkey,
            depositor_authority,
            10_000, // 100%
        )?;

        initialiazed_accounts.token_accounts.insert(
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

    initialiazed_accounts
        .save(&format!("accounts.{}.yaml", config.network))
        .unwrap();

    Ok(())
}

pub async fn command_cancel_withdraw_request(
    config: &Config,
    withdrawal_request_pubkey: &Pubkey,
) -> anyhow::Result<()> {
    let initialiazed_accounts = config.get_initialized_accounts();

    let withdrawal_request = config
        .get_account_unpack::<everlend_general_pool::state::WithdrawalRequest>(
            withdrawal_request_pubkey,
        )?;

    let general_pool = config
        .get_account_unpack::<everlend_general_pool::state::Pool>(&withdrawal_request.pool)?;

    general_pool::cancel_withdraw_request(
        config,
        &initialiazed_accounts.general_pool_market,
        &withdrawal_request.pool,
        &withdrawal_request.source,
        &general_pool.token_mint,
        &general_pool.pool_mint,
        &withdrawal_request.from,
    )?;

    Ok(())
}

pub async fn command_reset_rebalancing(
    config: &Config,
    rebalancing_pubkey: &Pubkey,
    amount_to_distribute: u64,
    distributed_liquidity: u64,
    distribution_vec: Vec<u64>,
) -> anyhow::Result<()> {
    let initialiazed_accounts = config.get_initialized_accounts();

    let rebalancing = config.get_account_unpack::<Rebalancing>(rebalancing_pubkey)?;
    let mut distribution_array = DistributionArray::default();
    distribution_array.copy_from_slice(distribution_vec.as_slice());

    println!("distribution_array {:?}", distribution_array);

    depositor::reset_rebalancing(
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

pub async fn command_add_reserve_liquidity(
    config: &Config,
    mint_key: &str,
    amount: u64,
) -> anyhow::Result<()> {
    let payer_pubkey = config.fee_payer.pubkey();
    let default_accounts = config.get_default_accounts();
    let initialiazed_accounts = config.get_initialized_accounts();

    let (mint_map, _) = get_asset_maps(default_accounts);
    let mint = mint_map.get(mint_key).unwrap();

    let (liquidity_reserve_transit_pubkey, _) = everlend_depositor::find_transit_program_address(
        &everlend_depositor::id(),
        &initialiazed_accounts.depositor,
        mint,
        "reserve",
    );

    println!(
        "liquidity_reserve_transit_pubkey = {:?}",
        liquidity_reserve_transit_pubkey
    );

    let token_account = get_associated_token_address(&payer_pubkey, mint);

    println!(
        "Transfer {} lamports of {} to reserve liquidity account",
        amount, mint_key
    );

    spl_token_transfer(
        config,
        &token_account,
        &liquidity_reserve_transit_pubkey,
        amount,
    )?;

    Ok(())
}

pub async fn command_info_reserve_liquidity(config: &Config) -> anyhow::Result<()> {
    let default_accounts = config.get_default_accounts();
    let initialiazed_accounts = config.get_initialized_accounts();

    let (mint_map, _) = get_asset_maps(default_accounts);

    for (_, mint) in mint_map.iter() {
        let (liquidity_reserve_transit_pubkey, _) =
            everlend_depositor::find_transit_program_address(
                &everlend_depositor::id(),
                &initialiazed_accounts.depositor,
                mint,
                "reserve",
            );

        let liquidity_reserve_transit = config
            .get_account_unpack::<spl_token::state::Account>(&liquidity_reserve_transit_pubkey)?;

        println!(
            "{:?}: {:?}",
            liquidity_reserve_transit_pubkey, liquidity_reserve_transit.amount
        );
    }

    Ok(())
}

async fn command_create(
    config: &Config,
    accounts_path: &str,
    required_mints: Vec<&str>,
    rebalance_executor: Pubkey,
) -> anyhow::Result<()> {
    let payer_pubkey = config.fee_payer.pubkey();
    println!("Fee payer: {}", payer_pubkey);

    let default_accounts = config.get_default_accounts();

    let (mint_map, collateral_mint_map) = get_asset_maps(default_accounts.clone());

    println!("Registry");
    let registry_pubkey = registry::init(config, None)?;
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

    registry::set_registry_config(
        config,
        &registry_pubkey,
        programs,
        RegistryRootAccounts::default(),
        RegistrySettings {
            refresh_income_interval: REFRESH_INCOME_INTERVAL,
        },
    )?;
    println!("programs = {:#?}", programs);

    let general_pool_market_pubkey = general_pool::create_market(config, None, &registry_pubkey)?;
    let income_pool_market_pubkey =
        income_pools::create_market(config, None, &general_pool_market_pubkey)?;

    let mm_collateral_pool_markets = vec![
        collateral_pool::create_market(config, None)?,
        collateral_pool::create_market(config, None)?,
        collateral_pool::create_market(config, None)?,
    ];

    println!("Liquidity oracle");
    let liquidity_oracle_pubkey = liquidity_oracle::init(config, None)?;
    let mut distribution = DistributionArray::default();
    distribution[0] = 0;
    distribution[1] = 0;
    distribution[2] = 0;

    println!("Registry");
    let registry_pubkey = registry::init(config, None)?;
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

    registry::set_registry_config(
        config,
        &registry_pubkey,
        programs,
        roots,
        RegistrySettings {
            refresh_income_interval: 0,
        },
    )?;

    println!("Depositor");
    let depositor_pubkey = depositor::init(config, &registry_pubkey, None, rebalance_executor)?;

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
            general_pool::create_pool(config, &general_pool_market_pubkey, mint)?;

        let token_account = get_associated_token_address(&payer_pubkey, mint);
        let pool_account =
            spl_create_associated_token_account(config, &payer_pubkey, &general_pool_mint)?;

        let (income_pool_pubkey, income_pool_token_account) =
            income_pools::create_pool(config, &income_pool_market_pubkey, mint)?;

        // MM Pools
        let mm_pool_collection = collateral_mints
            .iter()
            .map(
                |(collateral_mint, mm_pool_market_pubkey)| -> Result<PoolPubkeys, ClientError> {
                    println!("MM Pool: {}", collateral_mint);
                    let pool_pubkeys = collateral_pool::create_pool(
                        config,
                        mm_pool_market_pubkey,
                        collateral_mint,
                    )?;
                    collateral_pool::create_pool_withdraw_authority(
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

        liquidity_oracle::create_token_distribution(
            config,
            &liquidity_oracle_pubkey,
            mint,
            &distribution,
        )?;

        // Transit accounts
        let liquidity_transit_pubkey =
            depositor::create_transit(config, &depositor_pubkey, mint, None)?;

        // Reserve
        println!("Reserve transit");
        let liquidity_reserve_transit_pubkey = depositor::create_transit(
            config,
            &depositor_pubkey,
            mint,
            Some("reserve".to_string()),
        )?;
        spl_token_transfer(
            config,
            &token_account,
            &liquidity_reserve_transit_pubkey,
            10000,
        )?;

        collateral_mints
            .iter()
            .map(|(collateral_mint, _mm_pool_market_pubkey)| {
                depositor::create_transit(config, &depositor_pubkey, collateral_mint, None)
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
        general_pool::create_pool_borrow_authority(
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

async fn command_create_collateral_pools(
    config: &Config,
    accounts_path: &str,
) -> anyhow::Result<()> {
    let collateral_pool_markets = vec![
        collateral_pool::create_market(config, None)?,
        collateral_pool::create_market(config, None)?,
        collateral_pool::create_market(config, None)?,
    ];
    let mut initialized_accounts = InitializedAccounts::load(accounts_path).unwrap();
    initialized_accounts.collateral_pool_markets = collateral_pool_markets;

    let default_accounts = config.get_default_accounts();

    let (_, collateral_mint_map) = get_asset_maps(default_accounts.clone());

    let mut collateral_pool_markets: [Pubkey; TOTAL_DISTRIBUTIONS] = Default::default();
    collateral_pool_markets[..initialized_accounts.collateral_pool_markets.len()]
        .copy_from_slice(&initialized_accounts.collateral_pool_markets);

    let token_accounts = initialized_accounts.token_accounts.iter_mut();
    let depositor_pubkey = &initialized_accounts.depositor;
    for pair in token_accounts {
        let collateral_mints: Vec<(Pubkey, Pubkey)> = collateral_mint_map
            .get(pair.0)
            .unwrap()
            .iter()
            .zip(initialized_accounts.collateral_pool_markets.iter())
            .filter_map(|(collateral_mint, mm_pool_market_pubkey)| {
                collateral_mint.map(|coll_mint| (coll_mint, *mm_pool_market_pubkey))
            })
            .collect();

        let mm_pool_collection = collateral_mints
            .iter()
            .map(|(collateral_mint, mm_pool_market_pubkey)| {
                if !collateral_mint
                    .eq(&Pubkey::from_str("11111111111111111111111111111111").unwrap())
                {
                    println!("MM Pool: {}", collateral_mint);
                    collateral_pool::create_pool(config, mm_pool_market_pubkey, collateral_mint)
                } else {
                    Ok(PoolPubkeys {
                        pool: Pubkey::from_str("11111111111111111111111111111111").unwrap(),
                        token_account: Pubkey::from_str("11111111111111111111111111111111")
                            .unwrap(),
                    })
                }
            })
            .collect::<Result<Vec<PoolPubkeys>, ClientError>>()?;
        collateral_mints
            .iter()
            .map(|(collateral_mint, _mm_pool_market_pubkey)| {
                if !collateral_mint
                    .eq(&Pubkey::from_str("11111111111111111111111111111111").unwrap())
                {
                    depositor::create_transit(config, depositor_pubkey, collateral_mint, None)
                } else {
                    Ok(Pubkey::from_str("11111111111111111111111111111111").unwrap())
                }
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

        let mut accounts = pair.1;
        accounts.collateral_pools = collateral_pools;
    }
    initialized_accounts.save(accounts_path).unwrap();
    Ok(())
}

async fn create_pool_withdraw_authority(
    config: &Config,
    accounts_path: &str,
) -> anyhow::Result<()> {
    let mut initialized_accounts = InitializedAccounts::load(accounts_path).unwrap_or_default();
    let pool_markets = initialized_accounts.collateral_pool_markets;
    let depositor = initialized_accounts.depositor;
    let token_accounts = initialized_accounts.token_accounts.iter_mut();
    for pair in token_accounts {
        pair.1
            .collateral_pools
            .iter()
            .zip(pool_markets.clone())
            .filter(|(keyset, _)| {
                !keyset
                    .pool
                    .eq(&Pubkey::from_str("11111111111111111111111111111111").unwrap())
            })
            .map(|(keyset, market)| {
                let (depositor_authority, _) =
                    find_program_address(&everlend_depositor::id(), &depositor);
                collateral_pool::create_pool_withdraw_authority(
                    config,
                    &market,
                    &keyset.pool,
                    &depositor_authority,
                    &config.fee_payer.pubkey(),
                )
            })
            .collect::<Result<Vec<Pubkey>, ClientError>>()?;
    }
    Ok(())
}

async fn command_info(config: &Config, accounts_path: &str) -> anyhow::Result<()> {
    let initialized_accounts = InitializedAccounts::load(accounts_path).unwrap_or_default();
    let default_accounts = config.get_default_accounts();

    println!("fee_payer: {:?}", config.fee_payer.pubkey());
    println!("default_accounts = {:#?}", default_accounts);
    println!("{:#?}", initialized_accounts);

    println!(
        "{:#?}",
        get_general_pool_market(config, &initialized_accounts.general_pool_market)?
    );

    for (_, token_accounts) in initialized_accounts.token_accounts {
        println!("mint = {:?}", token_accounts.mint);
        let (withdraw_requests_pubkey, withdraw_requests) = get_withdrawal_requests(
            config,
            &initialized_accounts.general_pool_market,
            &token_accounts.mint,
        )?;
        println!("{:#?}", (withdraw_requests_pubkey, &withdraw_requests));

        let (rebalancing_pubkey, _) = find_rebalancing_program_address(
            &everlend_depositor::id(),
            &initialized_accounts.depositor,
            &token_accounts.mint,
        );

        let rebalancing = config.get_account_unpack::<Rebalancing>(&rebalancing_pubkey)?;
        println!("{:#?}", (rebalancing_pubkey, rebalancing));
    }

    Ok(())
}

fn command_run_migrate_pool_market(config: &Config, keypair: Keypair) -> anyhow::Result<()> {
    println!("Close general pool market");
    println!(
        "pool market id: {}",
        &config.initialized_accounts.general_pool_market
    );
    general_pool::close_pool_market_account(
        config,
        &config.initialized_accounts.general_pool_market,
    )?;
    println!("Closed general pool market");

    println!("Create general pool market");
    general_pool::create_market(config, Some(keypair), &config.initialized_accounts.registry)?;
    println!("Finished!");

    Ok(())
}

async fn command_create_depositor_transit_account(
    config: &Config,
    token_mint: Pubkey,
    seed: Option<String>,
) -> anyhow::Result<()> {
    let initialized_accounts = config.get_initialized_accounts();

    println!("Token mint {}. Seed {:?}", token_mint, seed);
    depositor::create_transit(config, &initialized_accounts.depositor, &token_mint, seed)?;

    Ok(())
}

// TODO remove after setup
async fn command_create_income_pool_safety_fund_token_account(
    config: &Config,
    accounts_path: &str,
    case: Option<String>,
) -> anyhow::Result<()> {
    let initialiazed_accounts = InitializedAccounts::load(accounts_path).unwrap_or_default();

    if case.is_none() {
        println!("Token mint not presented");
        return Ok(());
    }

    let token = initialiazed_accounts
        .token_accounts
        .get(&case.unwrap())
        .unwrap();

    println!("Create income pool safety fund token account");
    income_pools::create_income_pool_safety_fund_token_account(
        config,
        &initialiazed_accounts.income_pool_market,
        &token.mint,
    )?;
    println!("Finished!");

    Ok(())
}
