mod depositor;
mod income_pools;
mod liquidity_oracle;
mod registry;
mod ulp;
mod utils;

use everlend_depositor::{find_rebalancing_program_address, state::Rebalancing};
use everlend_liquidity_oracle::state::DistributionArray;
use everlend_registry::state::{SetRegistryConfigParams, TOTAL_DISTRIBUTIONS};
use everlend_utils::integrations::{self, MoneyMarketPubkeys};
use solana_client::rpc_client::RpcClient;
use solana_program::{program_pack::Pack, pubkey::Pubkey};
use solana_sdk::{
    commitment_config::CommitmentConfig, signature::read_keypair_file, signer::Signer,
};
use spl_associated_token_account::get_associated_token_address;
use std::str::FromStr;
use utils::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("E2E test");

    let config = {
        let cli_config = if let Some(ref config_file) = *solana_cli_config::CONFIG_FILE {
            solana_cli_config::Config::load(config_file).unwrap_or_default()
        } else {
            solana_cli_config::Config::default()
        };

        println!("{:#?}", cli_config);

        let json_rpc_url = cli_config.json_rpc_url;
        let fee_payer = read_keypair_file("owner_keypair.json").unwrap();

        Config {
            rpc_client: RpcClient::new_with_commitment(json_rpc_url, CommitmentConfig::confirmed()),
            verbose: true,
            fee_payer,
        }
    };

    solana_logger::setup_with_default("solana=info");

    let payer_pubkey = config.fee_payer.pubkey();
    println!("Fee payer: {}", payer_pubkey);

    println!("Running tests...");
    let sol_mint = Pubkey::from_str(SOL_MINT).unwrap();
    let sol_oracle = Pubkey::from_str(SOL_ORACLE).unwrap();
    let sol_larix_oracle = Pubkey::from_str(SOL_LARIX_ORACLE).unwrap();
    let port_finance_sol_collateral_mint =
        Pubkey::from_str(PORT_FINANCE_RESERVE_SOL_COLLATERAL_MINT).unwrap();
    let larix_sol_collateral_mint = Pubkey::from_str(LARIX_RESERVE_SOL_COLLATERAL_MINT).unwrap();
    let port_finance_program_id = Pubkey::from_str(integrations::PORT_FINANCE_PROGRAM_ID).unwrap();
    let larix_program_id = Pubkey::from_str(integrations::LARIX_PROGRAM_ID).unwrap();

    let port_finance_pubkeys = integrations::spl_token_lending::AccountPubkeys {
        reserve: Pubkey::from_str(PORT_FINANCE_RESERVE_SOL).unwrap(),
        reserve_liquidity_supply: Pubkey::from_str(PORT_FINANCE_RESERVE_SOL_SUPPLY).unwrap(),
        reserve_liquidity_oracle: sol_oracle,
        lending_market: Pubkey::from_str(PORT_FINANCE_LENDING_MARKET).unwrap(),
    };

    let larix_pubkeys = integrations::larix::AccountPubkeys {
        reserve: Pubkey::from_str(LARIX_RESERVE_SOL).unwrap(),
        reserve_liquidity_supply: Pubkey::from_str(LARIX_RESERVE_SOL_SUPPLY).unwrap(),
        reserve_liquidity_oracle: sol_oracle,
        reserve_larix_liquidity_oracle: sol_larix_oracle,
        lending_market: Pubkey::from_str(LARIX_LENDING_MARKET).unwrap(),
    };

    println!("0. Registry");
    let registry_pubkey = registry::init(&config, None)?;
    let mut registry_config = SetRegistryConfigParams {
        ulp_program_id: everlend_ulp::id(),
        liquidity_oracle_program_id: everlend_liquidity_oracle::id(),
        depositor_program_id: everlend_depositor::id(),
        income_pools_program_id: everlend_income_pools::id(),
        money_market_program_ids: [Pubkey::default(); TOTAL_DISTRIBUTIONS],
    };
    registry_config.money_market_program_ids[0] = port_finance_program_id;
    registry_config.money_market_program_ids[1] = larix_program_id;

    registry::set_registry_config(&config, &registry_pubkey, registry_config)?;

    println!("1. General pool");
    let pool_market_pubkey = ulp::create_market(&config, None)?;
    let (pool_pubkey, pool_token_account, pool_mint) =
        ulp::create_pool(&config, &pool_market_pubkey, &sol_mint)?;

    let token_account = get_associated_token_address(&payer_pubkey, &sol_mint);
    let pool_account = spl_create_associated_token_account(&config, &payer_pubkey, &pool_mint)?;

    println!("1.1. Income pool");
    let income_pool_market_pubkey =
        income_pools::create_market(&config, None, &pool_market_pubkey)?;
    let (income_pool_pubkey, income_pool_token_account) =
        income_pools::create_pool(&config, &income_pool_market_pubkey, &sol_mint)?;

    println!("2. Deposit liquidity");
    ulp::deposit(
        &config,
        &pool_market_pubkey,
        &pool_pubkey,
        &token_account,
        &pool_account,
        &pool_token_account,
        &pool_mint,
        1000,
    )?;

    println!("3. MM Pool: Port Finance");
    let port_finance_mm_pool_market_pubkey = ulp::create_market(&config, None)?;
    let (_mm_pool_pubkey, port_finance_mm_pool_token_account, port_finance_mm_pool_mint) =
        ulp::create_pool(
            &config,
            &port_finance_mm_pool_market_pubkey,
            &port_finance_sol_collateral_mint,
        )?;

    println!("3.1 MM Pool: Larix");
    let larix_mm_pool_market_pubkey = ulp::create_market(&config, None)?;
    let (_mm_pool_pubkey, larix_mm_pool_token_account, larix_mm_pool_mint) = ulp::create_pool(
        &config,
        &larix_mm_pool_market_pubkey,
        &larix_sol_collateral_mint,
    )?;

    println!("4. Liquidity oracle");
    let liquidity_oracle_pubkey = liquidity_oracle::init(&config, None)?;
    let mut distribution = DistributionArray::default();
    distribution[0] = 500_000_000u64;
    distribution[1] = 500_000_000u64;

    liquidity_oracle::create_token_distribution(
        &config,
        &liquidity_oracle_pubkey,
        &sol_mint,
        &distribution,
    )?;

    println!("5. Depositor");
    let depositor_pubkey = depositor::init(
        &config,
        None,
        &pool_market_pubkey,
        &income_pool_market_pubkey,
        &liquidity_oracle_pubkey,
    )?;

    let liquidity_transit_pubkey =
        depositor::create_transit(&config, &depositor_pubkey, &sol_mint)?;
    depositor::create_transit(
        &config,
        &depositor_pubkey,
        &port_finance_sol_collateral_mint,
    )?;
    depositor::create_transit(&config, &depositor_pubkey, &larix_sol_collateral_mint)?;
    depositor::create_transit(&config, &depositor_pubkey, &port_finance_mm_pool_mint)?;
    depositor::create_transit(&config, &depositor_pubkey, &larix_mm_pool_mint)?;

    println!("6. Prepare borrow authority");
    let (depositor_authority, _) =
        &everlend_utils::find_program_address(&everlend_depositor::id(), &depositor_pubkey);
    ulp::create_pool_borrow_authority(
        &config,
        &pool_market_pubkey,
        &pool_pubkey,
        depositor_authority,
        10_000, // 100%
    )?;

    println!("7. Rebalancing: Start");
    depositor::start_rebalancing(
        &config,
        &registry_pubkey,
        &depositor_pubkey,
        &sol_mint,
        &pool_market_pubkey,
        &pool_token_account,
        &liquidity_oracle_pubkey,
    )?;

    println!("7.1 Rebalancing: Deposit: Port Finance");
    depositor::deposit(
        &config,
        &registry_pubkey,
        &depositor_pubkey,
        &port_finance_mm_pool_market_pubkey,
        &port_finance_mm_pool_token_account,
        &sol_mint,
        &port_finance_sol_collateral_mint,
        &port_finance_mm_pool_mint,
        &port_finance_program_id,
        integrations::deposit_accounts(
            &port_finance_program_id,
            &MoneyMarketPubkeys::SPL(port_finance_pubkeys.clone()),
        ),
    )?;

    println!("7.2 Rebalancing: Deposit: Larix");
    depositor::deposit(
        &config,
        &registry_pubkey,
        &depositor_pubkey,
        &larix_mm_pool_market_pubkey,
        &larix_mm_pool_token_account,
        &sol_mint,
        &larix_sol_collateral_mint,
        &larix_mm_pool_mint,
        &larix_program_id,
        integrations::deposit_accounts(
            &larix_program_id,
            &MoneyMarketPubkeys::Larix(larix_pubkeys.clone()),
        ),
    )?;

    let mut balance = config
        .rpc_client
        .get_token_account_balance(&liquidity_transit_pubkey)?;
    println!("balance 0 = {:?}", balance);

    println!("8. Update token distribution");
    distribution[0] = 300_000_000u64; // 30%
    distribution[1] = 600_000_000u64; // 60%

    liquidity_oracle::update_token_distribution(
        &config,
        &liquidity_oracle_pubkey,
        &sol_mint,
        &distribution,
    )?;

    println!("8.1. Rebalancing: Start");
    let rebalancing_pubkey = depositor::start_rebalancing(
        &config,
        &registry_pubkey,
        &depositor_pubkey,
        &sol_mint,
        &pool_market_pubkey,
        &pool_token_account,
        &liquidity_oracle_pubkey,
    )?;
    let rebalancing_account = config.rpc_client.get_account(&rebalancing_pubkey)?;
    let rebalancing = Rebalancing::unpack(&rebalancing_account.data)?;

    println!("{:#?}", rebalancing);

    balance = config
        .rpc_client
        .get_token_account_balance(&liquidity_transit_pubkey)?;
    println!("balance 1 = {:?}", balance);

    println!("8.2. Rebalancing: Withdraw: Port Finance");
    depositor::withdraw(
        &config,
        &registry_pubkey,
        &depositor_pubkey,
        &income_pool_market_pubkey,
        &income_pool_token_account,
        &port_finance_mm_pool_market_pubkey,
        &port_finance_mm_pool_token_account,
        &port_finance_sol_collateral_mint,
        &sol_mint,
        &port_finance_mm_pool_mint,
        &port_finance_program_id,
        integrations::withdraw_accounts(
            &port_finance_program_id,
            &MoneyMarketPubkeys::SPL(port_finance_pubkeys.clone()),
        ),
    )?;

    balance = config
        .rpc_client
        .get_token_account_balance(&liquidity_transit_pubkey)?;

    println!("balance 2 = {:?}", balance);

    println!("8.3. Rebalancing: Deposit: Larix");
    depositor::deposit(
        &config,
        &registry_pubkey,
        &depositor_pubkey,
        &larix_mm_pool_market_pubkey,
        &larix_mm_pool_token_account,
        &sol_mint,
        &larix_sol_collateral_mint,
        &larix_mm_pool_mint,
        &larix_program_id,
        integrations::deposit_accounts(
            &larix_program_id,
            &MoneyMarketPubkeys::Larix(larix_pubkeys.clone()),
        ),
    )?;

    println!("9. Update token distribution");
    distribution[0] = 000_000_000u64; // 0%
    distribution[1] = 1_000_000_000u64; // 100%
    liquidity_oracle::update_token_distribution(
        &config,
        &liquidity_oracle_pubkey,
        &sol_mint,
        &distribution,
    )?;
    println!("9.1. Rebalancing: Start");
    depositor::start_rebalancing(
        &config,
        &registry_pubkey,
        &depositor_pubkey,
        &sol_mint,
        &pool_market_pubkey,
        &pool_token_account,
        &liquidity_oracle_pubkey,
    )?;

    let rebalancing_account = config.rpc_client.get_account(&rebalancing_pubkey)?;
    let rebalancing = Rebalancing::unpack(&rebalancing_account.data)?;

    println!("{:#?}", rebalancing);

    println!("9.2. Rebalancing: Withdraw: Port Finance");
    depositor::withdraw(
        &config,
        &registry_pubkey,
        &depositor_pubkey,
        &income_pool_market_pubkey,
        &income_pool_token_account,
        &port_finance_mm_pool_market_pubkey,
        &port_finance_mm_pool_token_account,
        &port_finance_sol_collateral_mint,
        &sol_mint,
        &port_finance_mm_pool_mint,
        &port_finance_program_id,
        integrations::withdraw_accounts(
            &port_finance_program_id,
            &MoneyMarketPubkeys::SPL(port_finance_pubkeys.clone()),
        ),
    )?;

    println!("9.3. Rebalancing: Deposit Larix");
    depositor::deposit(
        &config,
        &registry_pubkey,
        &depositor_pubkey,
        &larix_mm_pool_market_pubkey,
        &larix_mm_pool_token_account,
        &sol_mint,
        &larix_sol_collateral_mint,
        &larix_mm_pool_mint,
        &larix_program_id,
        integrations::deposit_accounts(
            &larix_program_id,
            &MoneyMarketPubkeys::Larix(larix_pubkeys.clone()),
        ),
    )?;

    println!("10. Update token distribution");
    distribution[0] = 100_000_000u64; // 10%
    distribution[1] = 0u64; // 0%
    liquidity_oracle::update_token_distribution(
        &config,
        &liquidity_oracle_pubkey,
        &sol_mint,
        &distribution,
    )?;
    println!("10.1. Rebalancing: Start");
    depositor::start_rebalancing(
        &config,
        &registry_pubkey,
        &depositor_pubkey,
        &sol_mint,
        &pool_market_pubkey,
        &pool_token_account,
        &liquidity_oracle_pubkey,
    )?;

    let rebalancing_account = config.rpc_client.get_account(&rebalancing_pubkey)?;
    let rebalancing = Rebalancing::unpack(&rebalancing_account.data)?;

    println!("{:#?}", rebalancing);

    println!("10.2. Rebalancing: Withdraw: Larix");
    depositor::withdraw(
        &config,
        &registry_pubkey,
        &depositor_pubkey,
        &income_pool_market_pubkey,
        &income_pool_token_account,
        &larix_mm_pool_market_pubkey,
        &larix_mm_pool_token_account,
        &larix_sol_collateral_mint,
        &sol_mint,
        &larix_mm_pool_mint,
        &larix_program_id,
        integrations::withdraw_accounts(
            &larix_program_id,
            &MoneyMarketPubkeys::Larix(larix_pubkeys),
        ),
    )?;

    println!("10.3. Rebalancing: Deposit Port Finance");
    depositor::deposit(
        &config,
        &registry_pubkey,
        &depositor_pubkey,
        &port_finance_mm_pool_market_pubkey,
        &port_finance_mm_pool_token_account,
        &sol_mint,
        &port_finance_sol_collateral_mint,
        &port_finance_mm_pool_mint,
        &port_finance_program_id,
        integrations::deposit_accounts(
            &port_finance_program_id,
            &MoneyMarketPubkeys::SPL(port_finance_pubkeys),
        ),
    )?;

    println!("Finished!");

    Ok(())
}
