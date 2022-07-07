use crate::utils::*;
use crate::MiningAccounts;
use anyhow::Result;
use everlend_depositor::{instruction::InitMiningAccountsPubkeys, state::MiningType};
use everlend_utils::integrations::MoneyMarket;
use solana_client::client_error::ClientError;
use solana_program::pubkey::Pubkey;
use solana_program::{program_pack::Pack, system_instruction};
use solana_sdk::{signature::Keypair, signer::Signer, transaction::Transaction};

pub mod larix_liquidity_miner;
pub mod larix_raw_test;
pub mod port_liquidity_miner;
pub mod quarry_liquidity_miner;
pub mod quarry_raw_test;

const LARIX_MINING_SIZE: u64 = 1 + 32 + 32 + 1 + 16 + 560;

pub fn init_token_account(
    config: &Config,
    account: &Keypair,
    token_mint: &Pubkey,
) -> Result<(), ClientError> {
    let rent = config
        .rpc_client
        .get_minimum_balance_for_rent_exemption(spl_token::state::Account::LEN as usize)?;
    let create_account_instruction = system_instruction::create_account(
        &config.fee_payer.pubkey(),
        &account.pubkey(),
        rent,
        spl_token::state::Account::LEN as u64,
        &spl_token::id(),
    );
    let init_account_instruction = spl_token::instruction::initialize_account(
        &spl_token::id(),
        &account.pubkey(),
        token_mint,
        &config.fee_payer.pubkey(),
    )
    .unwrap();
    let transaction = Transaction::new_with_payer(
        &[create_account_instruction, init_account_instruction],
        Some(&config.fee_payer.pubkey()),
    );
    config.sign_and_send_and_confirm_transaction(
        transaction,
        vec![config.fee_payer.as_ref(), account],
    )?;
    Ok(())
}

pub fn execute_mining_account_creation(
    config: &Config,
    staking_program_id: &Pubkey,
    mining_account: &Keypair,
    space: u64,
) -> Result<(), ClientError> {
    let rent = config
        .rpc_client
        .get_minimum_balance_for_rent_exemption(space as usize)?;
    let create_account_instruction = system_instruction::create_account(
        &config.fee_payer.pubkey(),
        &mining_account.pubkey(),
        rent,
        space,
        staking_program_id,
    );
    let tx = Transaction::new_with_payer(
        &[create_account_instruction],
        Some(&config.fee_payer.pubkey()),
    );
    config.sign_and_send_and_confirm_transaction(
        tx,
        vec![config.fee_payer.as_ref(), mining_account],
    )?;
    Ok(())
}

pub fn execute_init_mining_accounts(
    config: &Config,
    pubkeys: &InitMiningAccountsPubkeys,
    mining_type: MiningType,
) -> Result<()> {
    let init_mining_instruction = everlend_depositor::instruction::init_mining_accounts(
        &everlend_depositor::id(),
        pubkeys,
        mining_type,
    );
    let tx =
        Transaction::new_with_payer(&[init_mining_instruction], Some(&config.fee_payer.pubkey()));
    config.sign_and_send_and_confirm_transaction(tx, vec![config.fee_payer.as_ref()])?;
    Ok(())
}

pub fn save_mining_accounts(
    config: &Config,
    token: &String,
    money_market: MoneyMarket,
    mining_account: Pubkey,
) -> Result<()> {
    let default_accounts = config.get_default_accounts();
    let mut initialized_accounts = config.get_initialized_accounts();
    let (_, collateral_mint_map) = get_asset_maps(default_accounts);
    let collateral_mint = collateral_mint_map.get(token).unwrap()[money_market as usize].unwrap();
    // Generate internal mining account
    let (internal_mining_account, _) = everlend_depositor::find_internal_mining_program_address(
        &everlend_depositor::id(),
        &collateral_mint,
        &initialized_accounts.depositor,
    );
    // Save into account file
    initialized_accounts
        .token_accounts
        .get_mut(token)
        .unwrap()
        .mining_accounts[money_market as usize] = MiningAccounts {
        staking_account: mining_account,
        internal_mining_account,
    };

    initialized_accounts
        .save(&format!("accounts.{}.yaml", config.network))
        .unwrap();
    Ok(())
}

pub trait LiquidityMiner {
    fn get_mining_pubkey(&self, config: &Config, token: &String) -> Pubkey;
    fn save_mining_account_keypair(
        &self,
        config: &Config,
        token: &String,
        mining_account: &Keypair,
    ) -> Result<()>;
    fn create_mining_account(
        &self,
        config: &Config,
        token: &String,
        mining_account: &Keypair,
    ) -> Result<()>;
    fn get_pubkeys(&self, config: &Config, token: &String) -> Option<InitMiningAccountsPubkeys>;
    fn get_mining_type(
        &self,
        config: &Config,
        token: &String,
        mining_account: Pubkey,
    ) -> MiningType;
}

pub struct NotSupportedMiner {}

impl LiquidityMiner for NotSupportedMiner {
    fn create_mining_account(
        &self,
        _config: &Config,
        _token: &String,
        _keypair: &Keypair,
    ) -> Result<()> {
        Err(anyhow::anyhow!("Not implemented"))
    }
    fn save_mining_account_keypair(
        &self,
        _config: &Config,
        _token: &String,
        _mining_account: &Keypair,
    ) -> Result<()> {
        Err(anyhow::anyhow!("Not implemented"))
    }
    fn get_mining_pubkey(&self, _config: &Config, _token: &String) -> Pubkey {
        Pubkey::default()
    }
    fn get_pubkeys(&self, _config: &Config, _token: &String) -> Option<InitMiningAccountsPubkeys> {
        None
    }
    fn get_mining_type(
        &self,
        _config: &Config,
        _token: &String,
        _mining_account: Pubkey,
    ) -> MiningType {
        MiningType::None
    }
}
