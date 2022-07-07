use super::LiquidityMiner;
use crate::liquidity_mining::execute_mining_account_creation;
use crate::utils::*;
use anyhow::Result;
use everlend_depositor::{instruction::InitMiningAccountsPubkeys, state::MiningType};
use everlend_utils::integrations::{MoneyMarket, StakingMoneyMarket};
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;
use solana_sdk::signature::write_keypair_file;
use solana_sdk::{signature::Keypair, signer::Signer};

pub struct PortLiquidityMiner {}

fn save_new_mining_account(
    _config: &Config,
    token: &String,
    mining_account: &Keypair,
) -> Result<()> {
    write_keypair_file(
        &mining_account,
        &format!(
            ".keypairs/{}_port_mining_{}.json",
            token,
            mining_account.pubkey()
        ),
    )
    .unwrap();
    Ok(())
}

impl LiquidityMiner for PortLiquidityMiner {
    fn get_mining_pubkey(&self, config: &Config, token: &String) -> Pubkey {
        let mut initialized_accounts = config.get_initialized_accounts();
        initialized_accounts
            .token_accounts
            .get_mut(token)
            .unwrap()
            .mining_accounts[StakingMoneyMarket::PortFinance as usize]
            .staking_account
    }

    fn create_mining_account(
        &self,
        config: &Config,
        token: &String,
        mining_account: &Keypair,
    ) -> Result<()> {
        let default_accounts = config.get_default_accounts();
        println!("Create and Init port staking account");
        execute_mining_account_creation(
            config,
            &default_accounts.port_finance.staking_program_id,
            mining_account,
            port_finance_staking::state::stake_account::StakeAccount::LEN as u64,
        )?;
        save_new_mining_account(config, token, mining_account)?;
        Ok(())
    }

    fn get_pubkeys(&self, config: &Config, token: &String) -> Option<InitMiningAccountsPubkeys> {
        let initialized_accounts = config.get_initialized_accounts();
        let default_accounts = config.get_default_accounts();
        let (_, collateral_mint_map) = get_asset_maps(default_accounts.clone());
        let collateral_mint =
            collateral_mint_map.get(token).unwrap()[MoneyMarket::PortFinance as usize].unwrap();
        Some(InitMiningAccountsPubkeys {
            collateral_mint,
            depositor: initialized_accounts.depositor,
            registry: initialized_accounts.registry,
            manager: config.fee_payer.pubkey(),
            money_market_program_id: default_accounts.port_finance.program_id,
            lending_market: Some(default_accounts.port_finance.lending_market),
        })
    }

    fn get_mining_type(
        &self,
        config: &Config,
        token: &String,
        mining_account: Pubkey,
    ) -> MiningType {
        let default_accounts = config.get_default_accounts();
        let port_accounts = default_accounts.port_accounts.get(token).unwrap();
        MiningType::PortFinance {
            staking_program_id: default_accounts.port_finance.staking_program_id,
            staking_account: mining_account,
            staking_pool: port_accounts.staking_pool,
        }
    }

    fn update_mining_accounts(&self, _config: &Config) -> Result<()> {
        // No additional work needed for port
        Ok(())
    }
}
