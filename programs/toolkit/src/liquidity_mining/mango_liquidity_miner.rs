use solana_program::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use everlend_depositor::instruction::InitMiningAccountsPubkeys;
use everlend_depositor::state::MiningType;
use everlend_utils::integrations::MoneyMarket;
use crate::Config;
use crate::liquidity_mining::LiquidityMiner;
use crate::utils::get_asset_maps;

pub struct MangoLiquidityMiner {}

impl LiquidityMiner for MangoLiquidityMiner {
    fn get_mining_pubkey(&self, config: &Config, token: &String) -> Pubkey {
        let mut initialized_accounts = config.get_initialized_accounts();
        initialized_accounts
            .token_accounts
            .get_mut(token)
            .unwrap()
            .mining_accounts[MoneyMarket::Mango as usize]
            .staking_account
    }

    fn create_mining_account(
        &self,
        _config: &Config,
        _token: &String,
        _mining_account: &Keypair,
        _sub_reward_token_mint: Option<Pubkey>
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn get_pubkeys(&self, config: &Config, token: &String) -> Option<InitMiningAccountsPubkeys> {
        let initialized_accounts = config.get_initialized_accounts();
        let default_accounts = config.get_default_accounts();
        let (mint_map, collateral_mint_map) = get_asset_maps(default_accounts.clone());
        let liquidity_mint = mint_map.get(token).unwrap();
        let collateral_mint =
            collateral_mint_map.get(token).unwrap()[MoneyMarket::Mango as usize].unwrap();

        Some(InitMiningAccountsPubkeys {
            liquidity_mint: *liquidity_mint,
            collateral_mint,
            depositor: initialized_accounts.depositor,
            registry: initialized_accounts.registry,
            manager: config.fee_payer.pubkey(),
            money_market_program_id: default_accounts.port_finance.program_id,
            lending_market: Some(default_accounts.port_finance.lending_market),
        })
    }

    fn get_mining_type(
        &self,
        config: &Config,
        _token: &String,
        _mining_pubkey: Pubkey,
        _sub_reward_token_mint: Option<Pubkey>
    ) -> MiningType {
        let default_accounts = config.get_default_accounts();

        MiningType::Mango {
            staking_program_id: default_accounts.mango.program_id,
            mango_group: default_accounts.mango.mango_group
        }
    }
}