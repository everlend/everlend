#![allow(clippy::ptr_arg)]

use crate::utils::*;
use anyhow::Result;
use everlend_depositor::{instruction::InitMiningAccountsPubkeys, state::MiningType};
use everlend_utils::integrations::MoneyMarket;
use solana_client::client_error::ClientError;
use solana_program::pubkey::Pubkey;
use solana_program::system_instruction;
use solana_sdk::{signature::Keypair, signer::Signer, transaction::Transaction};

pub mod frakt_liquidity_miner;
pub mod larix_liquidity_miner;
pub mod larix_raw_test;
pub mod port_liquidity_miner;
pub mod quarry_liquidity_miner;
pub mod quarry_raw_test;

pub fn execute_account_creation(
    config: &Config,
    program_id: &Pubkey,
    account: &Keypair,
    space: u64,
) -> Result<(), ClientError> {
    let rent = config
        .rpc_client
        .get_minimum_balance_for_rent_exemption(space as usize)?;
    let create_account_instruction = system_instruction::create_account(
        &config.fee_payer.pubkey(),
        &account.pubkey(),
        rent,
        space,
        program_id,
    );
    let tx = Transaction::new_with_payer(
        &[create_account_instruction],
        Some(&config.fee_payer.pubkey()),
    );
    config.sign_and_send_and_confirm_transaction(tx, vec![config.fee_payer.as_ref(), account])?;
    Ok(())
}

pub fn execute_init_mining_accounts(
    config: &Config,
    pubkeys: &InitMiningAccountsPubkeys,
    mining_type: MiningType,
) -> Result<()> {
    let init_mining_instruction = everlend_depositor::instruction::init_mining_account(
        &everlend_depositor::id(),
        pubkeys,
        mining_type,
    );
    let tx =
        Transaction::new_with_payer(&[init_mining_instruction], Some(&config.fee_payer.pubkey()));
    config.sign_and_send_and_confirm_transaction(tx, vec![config.fee_payer.as_ref()])?;
    Ok(())
}

pub fn get_internal_mining_account(
    config: &Config,
    token: &String,
    money_market: MoneyMarket,
) -> Pubkey {
    let initialized_accounts = config.get_initialized_accounts();
    let default_accounts = config.get_default_accounts();
    let (mint_map, collateral_mint_map) = get_asset_maps(default_accounts);
    let liquidity_mint = mint_map.get(token).unwrap();
    let collateral_mint = collateral_mint_map.get(token).unwrap()[money_market as usize].unwrap();
    // Generate internal mining account
    let (internal_mining_account, _) = everlend_depositor::find_internal_mining_program_address(
        &everlend_depositor::id(),
        liquidity_mint,
        &collateral_mint,
        &initialized_accounts.depositor,
    );
    internal_mining_account
}

pub fn save_mining_accounts(
    config: &Config,
    token: &String,
    money_market: MoneyMarket,
) -> Result<()> {
    let mut initialized_accounts = config.get_initialized_accounts();
    let internal_mining_account = get_internal_mining_account(config, token, money_market);
    initialized_accounts
        .token_accounts
        .get_mut(token)
        .unwrap()
        .mining_accounts[money_market as usize]
        .internal_mining_account = internal_mining_account;
    initialized_accounts
        .save(config.accounts_path.as_str())
        .unwrap();
    Ok(())
}

pub trait LiquidityMiner {
    fn get_mining_pubkey(&self, config: &Config, token: &String) -> Pubkey;
    fn create_mining_account(
        &self,
        config: &Config,
        token: &String,
        mining_account: &Keypair,
        sub_reward_token_mint: Option<Pubkey>,
    ) -> Result<()>;
    fn get_pubkeys(&self, config: &Config, token: &String) -> Option<InitMiningAccountsPubkeys>;
    fn get_mining_type(
        &self,
        config: &Config,
        token: &String,
        mining_pubkey: Pubkey,
        sub_reward_token_mint: Option<Pubkey>,
    ) -> MiningType;
}
