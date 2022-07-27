use super::LiquidityMiner;
use crate::accounts_config::QuarryMining;
use crate::liquidity_mining::execute_account_creation;
use crate::utils::*;
use anyhow::Result;
use everlend_depositor::{instruction::InitMiningAccountsPubkeys, state::MiningType};
use everlend_utils::integrations::StakingMoneyMarket;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;
use solana_sdk::signature::write_keypair_file;
use solana_sdk::{signature::Keypair, signer::Signer};

pub struct QuarryLiquidityMiner {}

fn save_new_mining_account(
    config: &Config,
    token: &String,
    mining_account: &Keypair,
) -> Result<()> {
    write_keypair_file(
        &mining_account,
        &format!(
            ".keypairs/{}_quarry_mining_{}.json",
            token,
            mining_account.pubkey()
        ),
    )
    .unwrap();
    let mut initialized_accounts = config.get_initialized_accounts();
    let quarry_mining = initialized_accounts.quarry_mining.get_mut(token);
    if quarry_mining.is_none() {
        initialized_accounts
            .quarry_mining
            .insert(token.clone(), QuarryMining::default());
    }
    initialized_accounts
        .quarry_mining
        .get_mut(token)
        .unwrap()
        .miner_vault = mining_account.pubkey();
    initialized_accounts.save(&format!("accounts.{}.yaml", config.network))?;
    Ok(())
}

impl LiquidityMiner for QuarryLiquidityMiner {
    fn get_mining_pubkey(&self, config: &Config, token: &String) -> Pubkey {
        config
            .get_initialized_accounts()
            .quarry_mining
            .get_mut(token)
            .unwrap_or(&mut QuarryMining::default())
            .miner_vault
    }

    fn create_mining_account(
        &self,
        config: &Config,
        token: &String,
        mining_account: &Keypair,
        _sub_reward_token_mint: Option<Pubkey>,
    ) -> Result<()> {
        println!("Create and Init quarry mining accont");
        println!("Mining account: {}", mining_account.pubkey());
        execute_account_creation(
            config,
            &spl_token::id(),
            &mining_account,
            spl_token::state::Account::LEN as u64,
        )?;
        save_new_mining_account(config, token, &mining_account)?;
        Ok(())
    }

    fn get_pubkeys(&self, config: &Config, token: &String) -> Option<InitMiningAccountsPubkeys> {
        let default_accounts = config.get_default_accounts();
        let initialized_accounts = config.get_initialized_accounts();
        let (mint_map, collateral_mint_map) = get_asset_maps(default_accounts.clone());
        let liquidity_mint = mint_map.get(token).unwrap();
        let collateral_mint =
            collateral_mint_map.get(token).unwrap()[StakingMoneyMarket::Quarry as usize].unwrap();
        Some(InitMiningAccountsPubkeys {
            liquidity_mint: *liquidity_mint,
            collateral_mint,
            depositor: initialized_accounts.depositor,
            registry: initialized_accounts.registry,
            manager: config.fee_payer.pubkey(),
            money_market_program_id: default_accounts.quarry.mine_program_id,
            lending_market: None,
        })
    }

    fn get_mining_type(
        &self,
        config: &Config,
        _token: &String,
        mining_account: Pubkey,
        _sub_reward_token_mint: Option<Pubkey>,
    ) -> MiningType {
        let default_accounts = config.get_default_accounts();
        let quarry = default_accounts.quarry;
        MiningType::Quarry {
            quarry_mining_program_id: quarry.mine_program_id,
            quarry: quarry.quarry,
            rewarder: quarry.rewarder,
            miner_vault: mining_account,
        }
    }
}
