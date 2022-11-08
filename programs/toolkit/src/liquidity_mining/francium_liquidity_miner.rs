use crate::liquidity_mining::LiquidityMiner;
use crate::utils::get_asset_maps;
use crate::Config;
use anyhow::Result;
use everlend_depositor::instruction::InitMiningAccountsPubkeys;
use everlend_depositor::state::MiningType;
use everlend_utils::find_program_address;
use everlend_utils::integrations::MoneyMarket;
use solana_program::pubkey::Pubkey;

use solana_sdk::signature::Keypair;

use crate::helpers::create_transit;
use everlend_depositor::find_transit_program_address;
use everlend_utils::cpi::francium;

pub struct FranciumLiquidityMiner {}

fn save_new_mining_account(config: &Config, token: &String, mining_account: Pubkey) -> Result<()> {
    let mut initialized_accounts = config.get_initialized_accounts();

    initialized_accounts
        .token_accounts
        .get_mut(token)
        .unwrap()
        .mining_accounts[MoneyMarket::Francium as usize]
        .staking_account = mining_account;

    initialized_accounts
        .save(config.accounts_path.as_str())
        .unwrap();

    Ok(())
}

impl LiquidityMiner for FranciumLiquidityMiner {
    fn get_mining_pubkey(&self, config: &Config, token: &String) -> Pubkey {
        let initialized_accounts = config.get_initialized_accounts();
        initialized_accounts
            .token_accounts
            .get(token)
            .unwrap()
            .mining_accounts[MoneyMarket::Francium as usize]
            .staking_account
    }

    fn create_mining_account(
        &self,
        config: &Config,
        token: &String,
        _mining_account: &Keypair,
        sub_reward_token_mint: Option<Pubkey>,
        reward_token_mint: Option<Pubkey>,
    ) -> anyhow::Result<()> {
        let default_accounts = config.get_default_accounts();
        let initialized_accounts = config.get_initialized_accounts();
        let (depositor_authority, _) =
            find_program_address(&everlend_depositor::id(), &initialized_accounts.depositor);

        let (user_reward_a, _) = find_transit_program_address(
            &everlend_depositor::id(),
            &initialized_accounts.depositor,
            &reward_token_mint.unwrap(),
            francium::FRANCIUM_REWARD_SEED,
        );

        if config
            .rpc_client
            .get_token_account(&user_reward_a)
            .unwrap()
            .is_none()
        {
            create_transit(
                config,
                &initialized_accounts.depositor,
                &reward_token_mint.unwrap(),
                Some(francium::FRANCIUM_REWARD_SEED.to_string()),
            )?;
        }

        if sub_reward_token_mint.is_some() {
            let (user_reward_b, _) = find_transit_program_address(
                &everlend_depositor::id(),
                &initialized_accounts.depositor,
                &sub_reward_token_mint.unwrap(),
                francium::FRANCIUM_REWARD_SEED,
            );

            if config
                .rpc_client
                .get_token_account(&user_reward_b)
                .unwrap()
                .is_none()
            {
                create_transit(
                    config,
                    &initialized_accounts.depositor,
                    &sub_reward_token_mint.unwrap(),
                    Some(francium::FRANCIUM_REWARD_SEED.to_string()),
                )?;
            }
        }
        let (_, collateral_mint_map) = get_asset_maps(default_accounts.clone());
        let collateral_mint =
            collateral_mint_map.get(token).unwrap()[MoneyMarket::Francium as usize].unwrap();

        let (user_stake_account, _) = find_transit_program_address(
            &everlend_depositor::id(),
            &initialized_accounts.depositor,
            &collateral_mint,
            "",
        );

        let user_farming = francium::find_user_farming_address(
            &depositor_authority,
            default_accounts
                .francium_farming_pool_account
                .get(token)
                .unwrap(),
            &user_stake_account,
        );

        save_new_mining_account(config, token, user_farming)?;

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
        _mining_pubkey: Pubkey,
        sub_reward_token_mint: Option<Pubkey>,
        reward_token_mint: Option<Pubkey>,
    ) -> MiningType {
        let default_accounts = config.get_default_accounts();
        let initialized_accounts = config.get_initialized_accounts();

        let (_, collateral_mint_map) = get_asset_maps(default_accounts.clone());
        let collateral_mint =
            collateral_mint_map.get(token).unwrap()[MoneyMarket::Francium as usize].unwrap();

        let (user_stake_account, _) = find_transit_program_address(
            &everlend_depositor::id(),
            &initialized_accounts.depositor,
            &collateral_mint,
            "",
        );

        let (user_reward_a, _) = find_transit_program_address(
            &everlend_depositor::id(),
            &initialized_accounts.depositor,
            &reward_token_mint.unwrap(),
            francium::FRANCIUM_REWARD_SEED,
        );

        let (user_reward_b, _) = find_transit_program_address(
            &everlend_depositor::id(),
            &initialized_accounts.depositor,
            &sub_reward_token_mint.unwrap(),
            francium::FRANCIUM_REWARD_SEED,
        );

        MiningType::Francium {
            user_stake_token_account: user_stake_account,
            farming_pool: *default_accounts
                .francium_farming_pool_account
                .get(token)
                .unwrap(),
            user_reward_a,
            user_reward_b,
        }
    }
}
