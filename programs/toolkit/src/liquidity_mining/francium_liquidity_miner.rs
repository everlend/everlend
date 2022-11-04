use crate::liquidity_mining::{execute_account_creation, LiquidityMiner};
use crate::utils::{get_asset_maps, spl_create_associated_token_account};
use crate::Config;
use anyhow::Result;
use everlend_depositor::instruction::InitMiningAccountsPubkeys;
use everlend_depositor::state::MiningType;
use everlend_utils::find_program_address;
use everlend_utils::integrations::MoneyMarket;
use solana_program::pubkey::Pubkey;
use solana_sdk::signature::{write_keypair_file, Keypair};
use solana_sdk::signer::Signer;
use everlend_depositor::find_transit_program_address;

pub struct FranciumLiquidityMiner {}

const FARMING_POOL_SIZE: u64 = 530;

fn save_new_farming_pool_account(
    config: &Config,
    token: &String,
    farming_pool_account: &Keypair,
) -> Result<()> {
    write_keypair_file(
        farming_pool_account,
        &format!(
            ".keypairs/{}_francium_farming_{}.json",
            token,
            farming_pool_account.pubkey()
        ),
    )
        .unwrap();

    let mut initialized_accounts = config.get_initialized_accounts();

    initialized_accounts
        .token_accounts
        .get_mut(token)
        .unwrap()
        .francium_farming_pool_account = farming_pool_account.pubkey();

    initialized_accounts
        .save(config.accounts_path.as_str())
        .unwrap();
    Ok(())
}

fn save_new_mining_account(
    config: &Config,
    token: &String,
    mining_account: &Keypair,
) -> Result<()> {
    let mut initialized_accounts = config.get_initialized_accounts();
    write_keypair_file(
        mining_account,
        &format!(
            ".keypairs/{}_francium_mining_{}.json",
            token,
            mining_account.pubkey()
        ),
    )
        .unwrap();

    initialized_accounts
        .token_accounts
        .get_mut(token)
        .unwrap()
        .mining_accounts[MoneyMarket::Francium as usize]
        .staking_account = mining_account.pubkey();

    initialized_accounts
        .save(config.accounts_path.as_str())
        .unwrap();
    Ok(())
}

fn save_new_user_token_stake_account(
    config: &Config,
    token: &String,
    user_token_stake_account: &Keypair,
) -> Result<()> {
    let mut initialized_accounts = config.get_initialized_accounts();
    write_keypair_file(
        user_token_stake_account,
        &format!(
            ".keypairs/{}_francium_user_token_stake_{}.json",
            token,
            user_token_stake_account.pubkey()
        ),
    )
        .unwrap();

    initialized_accounts
        .token_accounts
        .get_mut(token)
        .unwrap()
        .francium_user_token_stake = user_token_stake_account.pubkey();

    initialized_accounts
        .save(config.accounts_path.as_str())
        .unwrap();
    Ok(())
}

impl LiquidityMiner for FranciumLiquidityMiner {
    fn get_mining_pubkey(&self, config: &Config, token: &String) -> Pubkey {
        let mut initialized_accounts = config.get_initialized_accounts();
        initialized_accounts
            .token_accounts
            .get_mut(token)
            .unwrap()
            .mining_accounts[MoneyMarket::Francium as usize]
            .staking_account
    }

    fn create_mining_account(
        &self,
        config: &Config,
        token: &String,
        mining_account: &Keypair,
        sub_reward_token_mint: Option<Pubkey>,
    ) -> anyhow::Result<()> {
        let default_accounts = config.get_default_accounts();
        let initialized_accounts = config.get_initialized_accounts();

        let (depositor_authority, _) = find_program_address(&everlend_depositor::id(), &initialized_accounts.depositor);
        if sub_reward_token_mint.is_some() {
            spl_create_associated_token_account(
                config,
                &depositor_authority,
                &sub_reward_token_mint.unwrap(),
            )?;
        }

        execute_account_creation(
            config,
            &default_accounts.francium.program_id,
            mining_account,
            FARMING_POOL_SIZE,
        )?;
        save_new_mining_account(config, token, mining_account)?;

        Ok(())
    }

    fn get_pubkeys(&self, config: &Config, token: &String) -> Option<InitMiningAccountsPubkeys> {
        let default_accounts = config.get_default_accounts();
        let initialized_accounts = config.get_initialized_accounts();
        let (mint_map, collateral_mint_map) = get_asset_maps(default_accounts.clone());
        let liquidity_mint = mint_map.get(token).unwrap();
        let collateral_mint =
            collateral_mint_map.get(token).unwrap()[MoneyMarket::Francium as usize].unwrap();
        Some(InitMiningAccountsPubkeys {
            liquidity_mint: *liquidity_mint,
            collateral_mint,
            depositor: initialized_accounts.depositor,
            registry: initialized_accounts.registry,
            manager: config.fee_payer.pubkey(),
            money_market_program_id: default_accounts.francium.program_id,
            lending_market: Some(default_accounts.francium.lending_market),
        })
    }

    fn get_mining_type(
        &self,
        config: &Config,
        token: &String,
        mining_pubkey: Pubkey,
        sub_reward_token_mint: Option<Pubkey>,
    ) -> MiningType {
        let default_accounts = config.get_default_accounts();
        let initialized_accounts = config.get_initialized_accounts();
        let token_accounts = initialized_accounts.token_accounts.get(token).unwrap();

        let (depositor_authority, _) = find_program_address(&everlend_depositor::id(), &initialized_accounts.depositor);

        let (user_reward_b, _ ) =
            find_transit_program_address(
                &default_accounts.francium.staking_program_id,
                &depositor_authority,
                &sub_reward_token_mint.unwrap(),
                "francium_reward"
            );

        MiningType::Francium {
            user_stake_token_account: token_accounts.francium_user_token_stake,
            farming_pool: token_accounts.francium_farming_pool_account,
            user_reward_a: mining_pubkey,
            user_reward_b,
        }
    }
}