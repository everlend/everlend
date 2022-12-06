use super::LiquidityMiner;
use crate::accounts_config::QuarryMining;
use crate::utils::*;
use anyhow::Result;
use everlend_depositor::{instruction::InitMiningAccountsPubkeys, state::MiningType};
use everlend_utils::cpi::quarry::{find_miner_program_address, find_quarry_program_address};
use everlend_utils::find_program_address;
use everlend_utils::integrations::MoneyMarket;
use solana_program::pubkey::Pubkey;
use solana_sdk::signature::Keypair;

pub struct QuarryLiquidityMiner {}

fn save_new_mining_account(config: &Config, token: &String, miner_vault: Pubkey) -> Result<()> {
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
        .miner_vault = miner_vault;
    initialized_accounts.save(config.accounts_path.as_str())?;
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
        _mining_account: &Keypair,
        _sub_reward_token_mint: Option<Pubkey>,
        _reward_token_mint: Option<Pubkey>,
    ) -> Result<()> {
        println!("Create and Init quarry mining accont");
        let default_accounts = config.get_default_accounts();
        let initialized_accounts = config.get_initialized_accounts();
        let (_, collateral_mint_map) = get_asset_maps(default_accounts.clone());

        let (depositor_authority, _) =
            find_program_address(&everlend_depositor::id(), &initialized_accounts.depositor);

        // Get by Port Finance index cause Quarry work now only with Port
        let collateral_mint =
            collateral_mint_map.get(token).unwrap()[MoneyMarket::PortFinance as usize].unwrap();

        let (quarry, _) = find_quarry_program_address(
            &default_accounts.quarry.mine_program_id,
            &default_accounts.quarry.rewarder,
            &collateral_mint,
        );
        println!("Quarry: {}", quarry);
        let (miner, _) = find_miner_program_address(
            &default_accounts.quarry.mine_program_id,
            &quarry,
            &depositor_authority,
        );
        println!("Miner: {}", miner);
        let miner_vault = spl_create_associated_token_account(config, &miner, &collateral_mint)?;

        println!("Miner vault: {}", miner_vault);
        save_new_mining_account(config, token, miner_vault)?;
        Ok(())
    }

    fn get_pubkeys(&self, config: &Config, token: &String) -> Option<InitMiningAccountsPubkeys> {
        let default_accounts = config.get_default_accounts();
        let initialized_accounts = config.get_initialized_accounts();
        let (mint_map, collateral_mint_map) = get_asset_maps(default_accounts.clone());
        let liquidity_mint = mint_map.get(token).unwrap();
        // Get by Port Finance index cause Quarry work now only with Port
        let collateral_mint =
            collateral_mint_map.get(token).unwrap()[MoneyMarket::PortFinance as usize].unwrap();
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
        _mining_account: Pubkey,
        _sub_reward_token_mint: Option<Pubkey>,
        _reward_token_mint: Option<Pubkey>,
    ) -> MiningType {
        let default_accounts = config.get_default_accounts();
        let quarry = default_accounts.quarry;
        MiningType::Quarry {
            rewarder: quarry.rewarder,
        }
    }
}
