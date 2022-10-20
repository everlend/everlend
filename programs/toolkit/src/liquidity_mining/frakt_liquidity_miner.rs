use crate::liquidity_mining::LiquidityMiner;
use crate::utils::get_asset_maps;
use crate::Config;
use everlend_depositor::instruction::InitMiningAccountsPubkeys;
use everlend_depositor::state::MiningType;
use everlend_utils::find_program_address;
use everlend_utils::integrations::MoneyMarket;
use solana_program::pubkey::Pubkey;
use solana_sdk::signature::Keypair;

pub struct FraktLiquidityMiner {}

impl LiquidityMiner for FraktLiquidityMiner {
    fn get_mining_pubkey(&self, config: &Config, token: &String) -> Pubkey {
        let mut initialized_accounts = config.get_initialized_accounts();
        initialized_accounts
            .token_accounts
            .get_mut(token)
            .unwrap()
            .mining_accounts[MoneyMarket::Frakt as usize]
            .staking_account
    }

    fn create_mining_account(
        &self,
        _config: &Config,
        _token: &String,
        _mining_account: &Keypair,
        _sub_reward_token_mint: Option<Pubkey>,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn get_pubkeys(&self, config: &Config, token: &String) -> Option<InitMiningAccountsPubkeys> {
        let default_accounts = config.get_default_accounts();
        let initialized_accounts = config.get_initialized_accounts();
        let (_, collateral_mint_map) = get_asset_maps(default_accounts.clone());
        let collateral_mint =
            collateral_mint_map.get(token).unwrap()[MoneyMarket::Frakt as usize].unwrap();

        Some(InitMiningAccountsPubkeys {
            liquidity_mint: default_accounts.sol_mint,
            collateral_mint,
            depositor: initialized_accounts.depositor,
            registry: initialized_accounts.registry,
            manager: config.fee_payer.pubkey(),
            money_market_program_id: default_accounts.frakt.program_id,
            lending_market: Some(default_accounts.frakt.liquidity_pool),
        })
    }

    fn get_mining_type(
        &self,
        config: &Config,
        _token: &String,
        _mining_pubkey: Pubkey,
        _sub_reward_token_mint: Option<Pubkey>,
    ) -> MiningType {
        let default_accounts = config.get_default_accounts();
        let initialized_accounts = config.get_initialized_accounts();
        let (depositor_authority, _) =
            find_program_address(&everlend_depositor::id(), &initialized_accounts.depositor);
        let (deposit_account, _) = everlend_utils::cpi::frakt::find_deposit_address(
            &default_accounts.frakt.program_id,
            &default_accounts.frakt.liquidity_pool,
            &depositor_authority,
        );

        MiningType::Frakt { deposit_account }
    }
}
