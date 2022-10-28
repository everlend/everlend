use crate::liquidity_mining::execute_account_creation;
use crate::utils::*;
use anyhow::Result;
use everlend_depositor::{instruction::InitMiningAccountsPubkeys, state::MiningType};
use everlend_utils::{find_program_address, integrations::MoneyMarket};
use solana_program::pubkey::Pubkey;
use solana_sdk::signature::write_keypair_file;
use solana_sdk::{signature::Keypair, signer::Signer};

use super::LiquidityMiner;

const LARIX_MINING_SIZE: u64 = 1 + 32 + 32 + 1 + 16 + 560;

pub struct LarixLiquidityMiner {}

fn save_new_mining_account(
    config: &Config,
    token: &String,
    mining_account: &Keypair,
) -> Result<()> {
    let mut initialized_accounts = config.get_initialized_accounts();
    write_keypair_file(
        mining_account,
        &format!(
            ".keypairs/{}_larix_mining_{}.json",
            token,
            mining_account.pubkey()
        ),
    )
    .unwrap();

    initialized_accounts
        .token_accounts
        .get_mut(token)
        .unwrap()
        .mining_accounts[MoneyMarket::Larix as usize]
        .staking_account = mining_account.pubkey();

    initialized_accounts
        .save(config.accounts_path.as_str())
        .unwrap();
    Ok(())
}

impl LiquidityMiner for LarixLiquidityMiner {
    fn get_mining_pubkey(&self, config: &Config, token: &String) -> Pubkey {
        let mut initialized_accounts = config.get_initialized_accounts();
        initialized_accounts
            .token_accounts
            .get_mut(token)
            .unwrap()
            .mining_accounts[MoneyMarket::Larix as usize]
            .staking_account
    }

    fn create_mining_account(
        &self,
        config: &Config,
        token: &String,
        mining_account: &Keypair,
        sub_reward_token_mint: Option<Pubkey>,
    ) -> Result<()> {
        let default_accounts = config.get_default_accounts();
        let initialized_accounts = config.get_initialized_accounts();
        println!("Create and Init larix mining accont");
        println!("Mining account: {}", mining_account.pubkey());

        let (depositor_authority, _) =
            find_program_address(&everlend_depositor::id(), &initialized_accounts.depositor);
        if sub_reward_token_mint.is_some() {
            spl_create_associated_token_account(
                config,
                &depositor_authority,
                &sub_reward_token_mint.unwrap(),
            )?;
        }

        execute_account_creation(
            config,
            &default_accounts.larix[0].program_id,
            mining_account,
            LARIX_MINING_SIZE,
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
            collateral_mint_map.get(token).unwrap()[MoneyMarket::Larix as usize].unwrap();
        Some(InitMiningAccountsPubkeys {
            liquidity_mint: *liquidity_mint,
            collateral_mint,
            depositor: initialized_accounts.depositor,
            registry: initialized_accounts.registry,
            manager: config.fee_payer.pubkey(),
            money_market_program_id: default_accounts.larix[0].program_id,
            lending_market: Some(default_accounts.larix[0].lending_market),
        })
    }

    fn get_mining_type(
        &self,
        config: &Config,
        _token: &String,
        mining_account: Pubkey,
        sub_reward_token_mint: Option<Pubkey>,
    ) -> MiningType {
        let initialized_accounts = config.get_initialized_accounts();

        let (depositor_authority, _) =
            find_program_address(&everlend_depositor::id(), &initialized_accounts.depositor);

        let additional_reward_token_account = sub_reward_token_mint.map(|sub_reward_token_mint| {
            spl_associated_token_account::get_associated_token_address(
                &depositor_authority,
                &sub_reward_token_mint,
            )
        });

        println!(
            "Additional reward token account {:?}",
            additional_reward_token_account
        );
        MiningType::Larix {
            mining_account,
            additional_reward_token_account,
        }
    }
}
