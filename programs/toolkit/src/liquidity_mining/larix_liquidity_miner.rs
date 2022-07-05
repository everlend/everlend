use super::LiquidityMiner;
use crate::accounts_config::LarixMiningAccount;
use crate::liquidity_mining::{execute_mining_account_creation, LARIX_MINING_SIZE};
use crate::utils::*;
use anyhow::Result;
use everlend_depositor::{instruction::InitMiningAccountsPubkeys, state::MiningType};
use everlend_utils::integrations::{MoneyMarket, StakingMoneyMarket};
use solana_program::pubkey::Pubkey;
use solana_sdk::signature::write_keypair_file;
use solana_sdk::{signature::Keypair, signer::Signer};

pub struct LarixLiquidityMiner {}

impl LiquidityMiner for LarixLiquidityMiner {
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
        self.save_mining_account_keypair(config, token, &mining_account)?;
        Ok(())
    }

    fn save_mining_account_keypair(
        &self,
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
        initialized_accounts.larix_mining.push(LarixMiningAccount {
            staking_account: mining_account.pubkey(),
            count: 0,
        });
        // Save into account file
        initialized_accounts
            .token_accounts
            .get_mut(token)
            .unwrap()
            .mining_accounts[MoneyMarket::Larix as usize]
            .staking_account = mining_account.pubkey();
        initialized_accounts
            .save(&format!("accounts.{}.yaml", config.network))
            .unwrap();
        Ok(())
    }

    fn get_mining_pubkey(&self, config: &Config, _token: &String) -> Pubkey {
        let larix_mining = config.get_initialized_accounts().larix_mining;
        larix_mining
            .last()
            .unwrap_or(&LarixMiningAccount {
                staking_account: Pubkey::default(),
                count: 0,
            })
            .staking_account
    }

    fn get_pubkeys(&self, config: &Config, token: &String) -> Option<InitMiningAccountsPubkeys> {
        let default_accounts = config.get_default_accounts();
        let initialized_accounts = config.get_initialized_accounts();
        let (_, collateral_mint_map) = get_asset_maps(default_accounts.clone());
        let collateral_mint =
            collateral_mint_map.get(token).unwrap()[StakingMoneyMarket::Larix as usize].unwrap();
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
        MiningType::Larix {
            mining_account: mining_account,
        }
    }
}
