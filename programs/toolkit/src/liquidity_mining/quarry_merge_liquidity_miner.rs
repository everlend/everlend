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

fn save_new_mining_account(
    config: &Config,
    token: &String,
    miner_vault_primary: Pubkey,
    miner_vault_replica: Pubkey,
) -> Result<()> {
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
        .miner_vault_primary = miner_vault_primary;
    initialized_accounts
        .quarry_merge_mining
        .get_mut(token)
        .unwrap()
        .miner_vault_replica = miner_vault_replica;

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
            .merge_miner
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
            collateral_mint_map.get(token).unwrap()[MoneyMarket::PortFinance as usize].unwrap();

        let (pool_pubkey, _) = quarry_merge::find_pool_program_address(
            &quarry_merge::staking_program_id(),
            &collateral_mint,
        );

        let (replica_mint, _) = quarry_merge::find_replica_mint_program_address(
            &quarry_merge::staking_program_id(),
            &pool_pubkey,
        );

        let (merge_miner_pubkey, _) = quarry_merge::find_merge_miner_program_address(
            &quarry_merge::staking_program_id(),
            &pool_pubkey,
            &depositor_authority,
        );

        let (quarry_primary, _) = quarry::find_quarry_program_address(
            &default_accounts.quarry.mine_program_id,
            &default_accounts.quarry_merge.rewarder_primary,
            &collateral_mint,
        );

        let (quarry_replica, _) = quarry::find_quarry_program_address(
            &default_accounts.quarry.mine_program_id,
            &default_accounts.quarry_merge.rewarder_replica,
            &replica_mint,
        );

        let (miner_primary, _) = quarry::find_miner_program_address(
            &default_accounts.quarry.mine_program_id,
            &quarry_primary,
            &merge_miner_pubkey,
        );

        let (miner_replica, _) = quarry::find_miner_program_address(
            &default_accounts.quarry.mine_program_id,
            &quarry_replica,
            &merge_miner_pubkey,
        );

        let miner_vault_primary =
            spl_create_associated_token_account(config, &miner_primary, &collateral_mint)?;
        let miner_vault_replica =
            spl_create_associated_token_account(config, &miner_replica, &replica_mint)?;

        save_new_mining_account(config, token, miner_vault_primary, miner_vault_replica)?;
        Ok(())
    }

    fn get_pubkeys(&self, config: &Config, token: &String) -> Option<InitMiningAccountsPubkeys> {
        let default_accounts = config.get_default_accounts();
        let initialized_accounts = config.get_initialized_accounts();
        let (mint_map, collateral_mint_map) = get_asset_maps(default_accounts.clone());
        let liquidity_mint = mint_map.get(token).unwrap();

        let collateral_mint =
            collateral_mint_map.get(token).unwrap()[MoneyMarket::PortFinance as usize].unwrap();
        Some(InitMiningAccountsPubkeys {
            liquidity_mint: *liquidity_mint,
            collateral_mint,
            depositor: initialized_accounts.depositor,
            registry: initialized_accounts.registry,
            manager: config.fee_payer.pubkey(),
            money_market_program_id: default_accounts.quarry_merge.merge_mine_program_id,
            lending_market: None,
        })
    }

    fn get_mining_type(
        &self,
        config: &Config,
        _token: &String,
        _mining_pubkey: Pubkey,
        _sub_reward_token_mint: Option<Pubkey>,
        _reward_token_mint: Option<Pubkey>,
    ) -> MiningType {
        let default_accounts = config.get_default_accounts();
        let quarry_merge = default_accounts.quarry_merge;

        MiningType::QuarryMerge {
            rewarder_primary: quarry_merge.rewarder_primary,
            rewarder_replica: quarry_merge.rewarder_replica,
        }
    }
}
