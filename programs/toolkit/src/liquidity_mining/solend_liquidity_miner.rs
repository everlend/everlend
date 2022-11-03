use crate::liquidity_mining::{execute_account_creation, LiquidityMiner};
use crate::utils::get_asset_maps;
use crate::Config;
use anyhow::Result;
use everlend_depositor::instruction::InitMiningAccountsPubkeys;
use everlend_depositor::state::MiningType;
use everlend_utils::integrations::MoneyMarket;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;
use solana_sdk::signature::{write_keypair_file, Keypair};
use solana_sdk::signer::Signer;

pub struct SolendLiquidityMiner {}

fn save_new_obligation_account(
    config: &Config,
    token: &String,
    obligation_account: &Keypair,
) -> Result<()> {
    write_keypair_file(
        obligation_account,
        &format!(
            ".keypairs/{}_solend_obligation_{}.json",
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
        .solend_obligation_account = obligation_account.pubkey();

    initialized_accounts
        .save(config.accounts_path.as_str())
        .unwrap();
    Ok(())
}

impl LiquidityMiner for SolendLiquidityMiner {
    fn get_mining_pubkey(&self, config: &Config, token: &String) -> Pubkey {
        let mut initialized_accounts = config.get_initialized_accounts();
        initialized_accounts
            .token_accounts
            .get_mut(token)
            .unwrap()
            .solend_obligation_account
    }

    fn create_mining_account(
        &self,
        config: &Config,
        token: &String,
        _mining_account: &Keypair,
        _sub_reward_token_mint: Option<Pubkey>,
    ) -> anyhow::Result<()> {
        let default_accounts = config.get_default_accounts();

        let obligation_account = Keypair::new();
        println!("Create Solend obligation account");
        execute_account_creation(
            config,
            &default_accounts.solend.program_id,
            &obligation_account,
            solend_program::state::Obligation::LEN as u64,
        )?;

        save_new_obligation_account(config, token, &obligation_account)?;

        Ok(())
    }

    fn get_pubkeys(&self, config: &Config, token: &String) -> Option<InitMiningAccountsPubkeys> {
        let initialized_accounts = config.get_initialized_accounts();
        let default_accounts = config.get_default_accounts();
        let (mint_map, collateral_mint_map) = get_asset_maps(default_accounts.clone());
        let liquidity_mint = mint_map.get(token).unwrap();
        let collateral_mint =
            collateral_mint_map.get(token).unwrap()[MoneyMarket::Solend as usize].unwrap();

        Some(InitMiningAccountsPubkeys {
            liquidity_mint: *liquidity_mint,
            collateral_mint,
            money_market_program_id: default_accounts.solend.program_id,
            depositor: initialized_accounts.depositor,
            registry: initialized_accounts.registry,
            manager: config.fee_payer.pubkey(),
            lending_market: Some(default_accounts.solend.lending_market),
        })
    }

    fn get_mining_type(
        &self,
        config: &Config,
        token: &String,
        _mining_pubkey: Pubkey,
        _sub_reward_token_mint: Option<Pubkey>,
    ) -> MiningType {
        let initialized_accounts = config.get_initialized_accounts();
        let token_accounts = initialized_accounts.token_accounts.get(token).unwrap();
        let obligation = token_accounts.solend_obligation_account;

        let solend_obligation_account = config.rpc_client.get_account(&obligation).unwrap();
        if solend_obligation_account.lamports == 0 {
            panic!("Uninitialized Obligation Account");
        }

        MiningType::Solend { obligation }
    }
}
