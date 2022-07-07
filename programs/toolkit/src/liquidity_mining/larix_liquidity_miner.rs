use super::{get_internal_mining_account, save_mining_accounts, LiquidityMiner};
use crate::accounts_config::{LarixMining, MiningAccounts};
use crate::liquidity_mining::execute_mining_account_creation;
use crate::utils::*;
use anyhow::Result;
use everlend_depositor::{instruction::InitMiningAccountsPubkeys, state::MiningType};
use everlend_utils::integrations::MoneyMarket;
use solana_program::pubkey::Pubkey;
use solana_sdk::signature::write_keypair_file;
use solana_sdk::{signature::Keypair, signer::Signer};

const LARIX_MINING_SIZE: u64 = 1 + 32 + 32 + 1 + 16 + 560;

pub struct LarixLiquidityMiner {}

fn save_new_mining_account(
    config: &Config,
    token: &String,
    mining_account: &Keypair,
) -> Result<()> {
    let mut initialized_accounts = config.get_initialized_accounts();
    write_keypair_file(
        &mining_account,
        &format!(
            ".keypairs/{}_larix_mining_{}.json",
            token,
            mining_account.pubkey()
        ),
    )
    .unwrap();
    // Larix can store up to 10 tokens on 1 account
    initialized_accounts.larix_mining.push(LarixMining {
        staking_account: mining_account.pubkey(),
        count: 0,
    });
    initialized_accounts
        .save(&format!("accounts.{}.yaml", config.network))
        .unwrap();
    Ok(())
}

impl LiquidityMiner for LarixLiquidityMiner {
    fn get_mining_pubkey(&self, config: &Config, _token: &String) -> Pubkey {
        let larix_mining = config.get_initialized_accounts().larix_mining;
        larix_mining
            .last()
            .unwrap_or(&LarixMining {
                staking_account: Pubkey::default(),
                count: 0,
            })
            .staking_account
    }

    fn create_mining_account(
        &self,
        config: &Config,
        token: &String,
        mining_account: &Keypair,
    ) -> Result<()> {
        let default_accounts = config.get_default_accounts();
        println!("Create and Init larix mining accont");
        println!("Mining account: {}", mining_account.pubkey());
        execute_mining_account_creation(
            config,
            &default_accounts.larix.program_id,
            &mining_account,
            LARIX_MINING_SIZE,
        )?;
        save_new_mining_account(config, token, mining_account)?;
        Ok(())
    }

    fn get_pubkeys(&self, config: &Config, token: &String) -> Option<InitMiningAccountsPubkeys> {
        let default_accounts = config.get_default_accounts();
        let initialized_accounts = config.get_initialized_accounts();
        let (_, collateral_mint_map) = get_asset_maps(default_accounts.clone());
        let collateral_mint =
            collateral_mint_map.get(token).unwrap()[MoneyMarket::Larix as usize].unwrap();
        Some(InitMiningAccountsPubkeys {
            collateral_mint,
            depositor: initialized_accounts.depositor,
            registry: initialized_accounts.registry,
            manager: config.fee_payer.pubkey(),
            money_market_program_id: default_accounts.larix.program_id,
            lending_market: Some(default_accounts.larix.lending_market),
        })
    }

    fn get_mining_type(
        &self,
        _config: &Config,
        _token: &String,
        mining_account: Pubkey,
    ) -> MiningType {
        MiningType::Larix { mining_account }
    }

    fn update_mining_accounts(&self, config: &Config) -> Result<()> {
        let mut initialized_accounts = config.get_initialized_accounts();
        let last_index = initialized_accounts.larix_mining.len() - 1;
        initialized_accounts.larix_mining[last_index].count += 1;
        initialized_accounts
            .save(&format!("accounts.{}.yaml", config.network))
            .unwrap();
        Ok(())
    }
}
