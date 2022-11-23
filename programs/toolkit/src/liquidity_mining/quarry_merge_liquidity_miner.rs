use crate::liquidity_mining::LiquidityMiner;
use crate::utils::{get_asset_maps, spl_create_associated_token_account};
use crate::Config;
use anyhow::Result;
use everlend_depositor::instruction::InitMiningAccountsPubkeys;
use everlend_depositor::state::MiningType;
use everlend_utils::find_program_address;
use everlend_utils::integrations::MoneyMarket;
use solana_program::pubkey::Pubkey;

use solana_sdk::signature::Keypair;

use crate::accounts_config::QuarryMergeMining;
use everlend_utils::cpi::{quarry, quarry_merge};

pub struct QuarryMergeLiquidityMiner {}

fn save_new_mining_account(config: &Config, token: &String, miner_vault: Pubkey) -> Result<()> {
    let mut initialized_accounts = config.get_initialized_accounts();
    let quarry_merge_mining = initialized_accounts.quarry_merge_mining.get_mut(token);
    if quarry_merge_mining.is_none() {
        initialized_accounts
            .quarry_merge_mining
            .insert(token.clone(), QuarryMergeMining::default());
    }

    initialized_accounts
        .quarry_merge_mining
        .get_mut(token)
        .unwrap()
        .miner_vault = miner_vault;
    initialized_accounts.save(config.accounts_path.as_str())?;
    Ok(())
}

impl LiquidityMiner for QuarryMergeLiquidityMiner {
    fn get_mining_pubkey(&self, config: &Config, token: &String) -> Pubkey {
        config
            .get_initialized_accounts()
            .quarry_merge_mining
            .get_mut(token)
            .unwrap_or(&mut QuarryMergeMining::default())
            .miner_vault
    }

    fn create_mining_account(
        &self,
        config: &Config,
        token: &String,
        _mining_account: &Keypair,
        _sub_reward_token_mint: Option<Pubkey>,
        _reward_token_mint: Option<Pubkey>,
    ) -> anyhow::Result<()> {
        let default_accounts = config.get_default_accounts();
        let initialized_accounts = config.get_initialized_accounts();
        let (_, collateral_mint_map) = get_asset_maps(default_accounts.clone());

        let (depositor_authority, _) =
            find_program_address(&everlend_depositor::id(), &initialized_accounts.depositor);

        let collateral_mint =
            collateral_mint_map.get(token).unwrap()[MoneyMarket::QuarryMerge as usize].unwrap();

        let (pool_pubkey, _) = quarry_merge::find_pool_program_address(
            &quarry_merge::staking_program_id(),
            &collateral_mint,
        );

        let (merge_miner_pubkey, _) = quarry_merge::find_merge_miner_program_address(
            &quarry_merge::staking_program_id(),
            &pool_pubkey,
            &depositor_authority,
        );

        let (quarry, _) = quarry::find_quarry_program_address(
            &default_accounts.quarry.mine_program_id,
            &default_accounts.quarry_merge.rewarder,
            &collateral_mint,
        );

        let (miner, _) = quarry::find_miner_program_address(
            &default_accounts.quarry.mine_program_id,
            &quarry,
            &merge_miner_pubkey,
        );

        let miner_vault = spl_create_associated_token_account(config, &miner, &collateral_mint)?;

        save_new_mining_account(config, token, miner_vault)?;
        Ok(())
    }

    fn get_pubkeys(&self, config: &Config, token: &String) -> Option<InitMiningAccountsPubkeys> {
        let default_accounts = config.get_default_accounts();
        let initialized_accounts = config.get_initialized_accounts();
        let (mint_map, collateral_mint_map) = get_asset_maps(default_accounts.clone());
        let liquidity_mint = mint_map.get(token).unwrap();

        let collateral_mint =
            collateral_mint_map.get(token).unwrap()[MoneyMarket::QuarryMerge as usize].unwrap();
        Some(InitMiningAccountsPubkeys {
            liquidity_mint: *liquidity_mint,
            collateral_mint,
            depositor: initialized_accounts.depositor,
            registry: initialized_accounts.registry,
            manager: config.fee_payer.pubkey(),
            money_market_program_id: default_accounts.quarry_merge.mine_program_id,
            lending_market: None,
        })
    }

    fn get_mining_type(
        &self,
        config: &Config,
        token: &String,
        _mining_pubkey: Pubkey,
        _sub_reward_token_mint: Option<Pubkey>,
        _reward_token_mint: Option<Pubkey>,
    ) -> MiningType {
        let default_accounts = config.get_default_accounts();

        let (_, collateral_mint_map) = get_asset_maps(default_accounts.clone());
        let collateral_mint =
            collateral_mint_map.get(token).unwrap()[MoneyMarket::QuarryMerge as usize].unwrap();

        let (pool, _) = quarry_merge::find_pool_program_address(
            &quarry_merge::staking_program_id(),
            &collateral_mint,
        );

        let quarry_merge = default_accounts.quarry_merge;

        MiningType::QuarryMerge {
            pool,
            rewarder: quarry_merge.rewarder,
        }
    }
}
