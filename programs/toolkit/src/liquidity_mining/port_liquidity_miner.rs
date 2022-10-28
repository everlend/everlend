use super::LiquidityMiner;
use crate::helpers::create_transit;
use crate::liquidity_mining::execute_account_creation;
use crate::utils::*;
use anyhow::Result;
use everlend_depositor::{instruction::InitMiningAccountsPubkeys, state::MiningType};
use everlend_utils::integrations::MoneyMarket;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;
use solana_sdk::signature::write_keypair_file;
use solana_sdk::{signature::Keypair, signer::Signer};

pub struct PortLiquidityMiner {}

fn save_new_mining_account(
    config: &Config,
    token: &String,
    mining_account: &Keypair,
) -> Result<()> {
    write_keypair_file(
        mining_account,
        &format!(
            ".keypairs/{}_port_mining_{}.json",
            token,
            mining_account.pubkey()
        ),
    )
    .unwrap();

    let mut initialized_accounts = config.get_initialized_accounts();

    initialized_accounts
        .token_accounts
        .get_mut(token)
        .unwrap()
        .mining_accounts[MoneyMarket::PortFinance as usize]
        .staking_account = mining_account.pubkey();

    initialized_accounts
        .save(config.accounts_path.as_str())
        .unwrap();
    Ok(())
}

fn save_new_obligation_account(
    config: &Config,
    token: &String,
    obligation_account: &Keypair,
) -> Result<()> {
    write_keypair_file(
        obligation_account,
        &format!(
            ".keypairs/{}_port_obligation_{}.json",
            token,
            obligation_account.pubkey()
        ),
    )
    .unwrap();

    let mut initialized_accounts = config.get_initialized_accounts();

    initialized_accounts
        .token_accounts
        .get_mut(token)
        .unwrap()
        .port_finance_obligation_account = obligation_account.pubkey();

    initialized_accounts
        .save(config.accounts_path.as_str())
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
            .mining_accounts[MoneyMarket::PortFinance as usize]
            .staking_account
    }

    fn create_mining_account(
        &self,
        config: &Config,
        token: &String,
        mining_account: &Keypair,
        sub_reward_token_mint: Option<Pubkey>,
    ) -> Result<()> {
        let initialized_accounts = config.get_initialized_accounts();
        let token_accounts = initialized_accounts.token_accounts.get(token).unwrap();
        let default_accounts = config.get_default_accounts();

        if token_accounts
            .port_finance_obligation_account
            .eq(&Pubkey::default())
        {
            let obligation_account = Keypair::new();
            println!("Create port obligation account");
            execute_account_creation(
                config,
                &default_accounts.port_finance[0].program_id,
                &obligation_account,
                port_variable_rate_lending_instructions::state::Obligation::LEN as u64,
            )?;

            save_new_obligation_account(config, token, &obligation_account)?;
        }

        if sub_reward_token_mint.is_some() {
            create_transit(
                config,
                &initialized_accounts.depositor,
                &sub_reward_token_mint.unwrap(),
                Some("lm_reward".to_owned()),
            )?;
        };

        println!("Create and Init port staking account");
        execute_account_creation(
            config,
            &default_accounts.port_finance[0].staking_program_id,
            mining_account,
            port_finance_staking::state::stake_account::StakeAccount::LEN as u64,
        )?;
        save_new_mining_account(config, token, mining_account)?;
        Ok(())
    }

    fn get_pubkeys(&self, config: &Config, token: &String) -> Option<InitMiningAccountsPubkeys> {
        let initialized_accounts = config.get_initialized_accounts();
        let default_accounts = config.get_default_accounts();
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
            money_market_program_id: default_accounts.port_finance[0].program_id,
            lending_market: Some(default_accounts.port_finance[0].lending_market),
        })
    }

    fn get_mining_type(
        &self,
        config: &Config,
        token: &String,
        mining_account: Pubkey,
        _sub_reward_token_mint: Option<Pubkey>,
    ) -> MiningType {
        let default_accounts = config.get_default_accounts();
        let port_accounts = default_accounts.port_accounts.get(token).unwrap();
        let initialized_accounts = config.get_initialized_accounts();
        let token_accounts = initialized_accounts.token_accounts.get(token).unwrap();

        MiningType::PortFinance {
            staking_program_id: default_accounts.port_finance[0].staking_program_id,
            staking_account: mining_account,
            staking_pool: port_accounts.staking_pool,
            obligation: token_accounts.port_finance_obligation_account,
        }
    }
}
