mod depositor;
mod liquidity_oracle;
mod ulp;
mod utils;

use everlend_liquidity_oracle::state::{DistributionArray, LiquidityDistribution};
use everlend_utils::integrations::{self, MoneyMarketPubkeys};
use solana_client::rpc_client::RpcClient;
use solana_program::pubkey::Pubkey;
use solana_sdk::{
    commitment_config::CommitmentConfig, signature::read_keypair_file, signer::Signer,
};
use spl_associated_token_account::get_associated_token_address;
use std::str::FromStr;
use utils::*;


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // TODO: Make tests async + process result.
    let _result = port_finance_e2e().await;
    let _result = larix_e2e().await;
    Ok(())
}

async fn port_finance_e2e() -> anyhow::Result<()> {
    println!("Port finance E2E tests");

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
    let sol_collateral_mint = Pubkey::from_str(PORT_FINANCE_RESERVE_SOL_COLLATERAL_MINT).unwrap();
    let port_finance_program_id = Pubkey::from_str(integrations::PORT_FINANCE_PROGRAM_ID).unwrap();

    let port_finance_pubkeys = integrations::spl_token_lending::AccountPubkeys {
        reserve: Pubkey::from_str(PORT_FINANCE_RESERVE_SOL).unwrap(),
        reserve_liquidity_supply: Pubkey::from_str(PORT_FINANCE_RESERVE_SOL_SUPPLY).unwrap(),
        reserve_liquidity_oracle: sol_oracle,
        lending_market: Pubkey::from_str(PORT_FINANCE_LENDING_MARKET).unwrap(),
    };

    println!("1. General pool");
    let pool_market_pubkey = ulp::create_market(&config, None)?;
    let (pool_pubkey, pool_token_account, pool_mint) =
        ulp::create_pool(&config, &pool_market_pubkey, &sol_mint)?;

    let token_account = get_associated_token_address(&payer_pubkey, &sol_mint);
    let pool_account = spl_create_associated_token_account(&config, &payer_pubkey, &pool_mint)?;

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

    println!("3. MM Pool");
    let mm_pool_market_pubkey = ulp::create_market(&config, None)?;
    let (_mm_pool_pubkey, mm_pool_token_account, mm_pool_mint) =
        ulp::create_pool(&config, &mm_pool_market_pubkey, &sol_collateral_mint)?;

    println!("4. Liquidity oracle");
    let liquidity_oracle_pubkey = liquidity_oracle::init(&config, None)?;
    let mut distribution = DistributionArray::default();
    distribution[0] = LiquidityDistribution {
        money_market: port_finance_program_id,
        percent: 500_000_000u64, // 50%
    };
    liquidity_oracle::create_token_distribution(
        &config,
        &liquidity_oracle_pubkey,
        &sol_mint,
        &distribution,
    )?;

    println!("5. Depositor");
    let depositor_pubkey =
        depositor::init(&config, None, &pool_market_pubkey, &liquidity_oracle_pubkey)?;

    depositor::create_transit(&config, &depositor_pubkey, &sol_mint)?;
    depositor::create_transit(&config, &depositor_pubkey, &sol_collateral_mint)?;
    depositor::create_transit(&config, &depositor_pubkey, &mm_pool_mint)?;

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
        &depositor_pubkey,
        &sol_mint,
        &pool_market_pubkey,
        &pool_token_account,
        &liquidity_oracle_pubkey,
    )?;

    println!("7.1 Rebalancing: Deposit");
    let deposit_accounts = integrations::deposit_accounts(
        &port_finance_program_id,
        &MoneyMarketPubkeys::SPL(port_finance_pubkeys.clone()),
    );

    depositor::deposit(
        &config,
        &depositor_pubkey,
        &pool_market_pubkey,
        &pool_token_account,
        &mm_pool_market_pubkey,
        &mm_pool_token_account,
        &sol_mint,
        &sol_collateral_mint,
        &mm_pool_mint,
        &port_finance_program_id,
        deposit_accounts,
        // 500,
    )?;

    println!("8. Update token distribution");
    distribution[0].percent = 300_000_000u64; // 30%
    liquidity_oracle::update_token_distribution(
        &config,
        &liquidity_oracle_pubkey,
        &sol_mint,
        &distribution,
    )?;
    println!("8.1. Rebalancing: Start");
    depositor::start_rebalancing(
        &config,
        &depositor_pubkey,
        &sol_mint,
        &pool_market_pubkey,
        &pool_token_account,
        &liquidity_oracle_pubkey,
    )?;
    println!("8.2. Rebalancing: Withdraw");
    let withdraw_accounts = integrations::withdraw_accounts(
        &port_finance_program_id,
        &MoneyMarketPubkeys::SPL(port_finance_pubkeys.clone()),
    );
    depositor::withdraw(
        &config,
        &depositor_pubkey,
        &pool_market_pubkey,
        &pool_token_account,
        &mm_pool_market_pubkey,
        &mm_pool_token_account,
        &sol_collateral_mint,
        &sol_mint,
        &mm_pool_mint,
        &port_finance_program_id,
        withdraw_accounts,
        // 200,
    )?;

    println!("9. Update token distribution");
    distribution[0].percent = 000_000_000u64; // 30%
    liquidity_oracle::update_token_distribution(
        &config,
        &liquidity_oracle_pubkey,
        &sol_mint,
        &distribution,
    )?;
    println!("9.1. Rebalancing: Start");
    depositor::start_rebalancing(
        &config,
        &depositor_pubkey,
        &sol_mint,
        &pool_market_pubkey,
        &pool_token_account,
        &liquidity_oracle_pubkey,
    )?;
    println!("9.2. Rebalancing: Withdraw");
    let withdraw_accounts = integrations::withdraw_accounts(
        &port_finance_program_id,
        &MoneyMarketPubkeys::SPL(port_finance_pubkeys),
    );
    depositor::withdraw(
        &config,
        &depositor_pubkey,
        &pool_market_pubkey,
        &pool_token_account,
        &mm_pool_market_pubkey,
        &mm_pool_token_account,
        &sol_collateral_mint,
        &sol_mint,
        &mm_pool_mint,
        &port_finance_program_id,
        withdraw_accounts,
        // 300,
    )?;

    println!("Finished!");

    Ok(())
}

async fn larix_e2e() -> anyhow::Result<()> {
    println!("Larix E2E tests");

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
    let sol_collateral_mint = Pubkey::from_str(LARIX_RESERVE_SOL_COLLATERAL_MINT).unwrap();
    let larix_program_id = Pubkey::from_str(integrations::LARIX_PROGRAM_ID).unwrap();

    let larix_pubkeys = integrations::larix::AccountPubkeys {
        reserve: Pubkey::from_str(LARIX_RESERVE_SOL).unwrap(),
        reserve_liquidity_supply: Pubkey::from_str(LARIX_RESERVE_SOL_SUPPLY).unwrap(),
        reserve_liquidity_oracle: sol_oracle,
        reserve_larix_liquidity_oracle: sol_larix_oracle,
        lending_market: Pubkey::from_str(LARIX_LENDING_MARKET).unwrap(),
    };

    println!("1. General pool");
    let pool_market_pubkey = ulp::create_market(&config, None)?;
    let (pool_pubkey, pool_token_account, pool_mint) =
        ulp::create_pool(&config, &pool_market_pubkey, &sol_mint)?;

    let token_account = get_associated_token_address(&payer_pubkey, &sol_mint);
    let pool_account = spl_create_associated_token_account(&config, &payer_pubkey, &pool_mint)?;

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

    println!("3. MM Pool");
    let mm_pool_market_pubkey = ulp::create_market(&config, None)?;

    println!("3.2 MM Pool create_pool");
    let (_mm_pool_pubkey, mm_pool_token_account, mm_pool_mint) =
        ulp::create_pool(&config, &mm_pool_market_pubkey, &sol_collateral_mint)?;

    println!("4. Liquidity oracle");
    let liquidity_oracle_pubkey = liquidity_oracle::init(&config, None)?;
    let mut distribution = DistributionArray::default();
    distribution[0] = LiquidityDistribution {
        money_market: larix_program_id,
        percent: 500_000_000u64, // 50%
    };
    liquidity_oracle::create_token_distribution(
        &config,
        &liquidity_oracle_pubkey,
        &sol_mint,
        &distribution,
    )?;

    println!("5. Depositor");
    let depositor_pubkey =
        depositor::init(&config, None, &pool_market_pubkey, &liquidity_oracle_pubkey)?;

    depositor::create_transit(&config, &depositor_pubkey, &sol_mint)?;
    depositor::create_transit(&config, &depositor_pubkey, &sol_collateral_mint)?;
    depositor::create_transit(&config, &depositor_pubkey, &mm_pool_mint)?;

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
        &depositor_pubkey,
        &sol_mint,
        &pool_market_pubkey,
        &pool_token_account,
        &liquidity_oracle_pubkey,
    )?;

    println!("7.1 Rebalancing: Deposit");
    let deposit_accounts = integrations::deposit_accounts(
        &larix_program_id,
        &MoneyMarketPubkeys::Larix(larix_pubkeys.clone()),
    );

    depositor::deposit(
        &config,
        &depositor_pubkey,
        &pool_market_pubkey,
        &pool_token_account,
        &mm_pool_market_pubkey,
        &mm_pool_token_account,
        &sol_mint,
        &sol_collateral_mint,
        &mm_pool_mint,
        &larix_program_id,
        deposit_accounts,
        500,
    )?;

    println!("8. Update token distribution");
    distribution[0].percent = 300_000_000u64; // 30%
    liquidity_oracle::update_token_distribution(
        &config,
        &liquidity_oracle_pubkey,
        &sol_mint,
        &distribution,
    )?;
    println!("8.1. Rebalancing: Start");
    depositor::start_rebalancing(
        &config,
        &depositor_pubkey,
        &sol_mint,
        &pool_market_pubkey,
        &pool_token_account,
        &liquidity_oracle_pubkey,
    )?;
    println!("8.2. Rebalancing: Withdraw");
    let withdraw_accounts = integrations::withdraw_accounts(
        &larix_program_id,
        &MoneyMarketPubkeys::Larix(larix_pubkeys),
    );
    depositor::withdraw(
        &config,
        &depositor_pubkey,
        &pool_market_pubkey,
        &pool_token_account,
        &mm_pool_market_pubkey,
        &mm_pool_token_account,
        &sol_collateral_mint,
        &sol_mint,
        &mm_pool_mint,
        &larix_program_id,
        withdraw_accounts,
        200,
    )?;

    println!("Finished!");

    Ok(())
}
