use std::collections::HashMap;

use anchor_lang::AccountDeserialize;
use solana_account_decoder::UiAccountEncoding;
use solana_client::{
    client_error::ClientError,
    rpc_client::RpcClient,
    rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
    rpc_filter::{Memcmp, MemcmpEncodedBytes, MemcmpEncoding, RpcFilterType},
};
use solana_program::{
    program_pack::{IsInitialized, Pack},
    pubkey::Pubkey,
};
use solana_sdk::{
    account::Account, signature::Signature, signer::Signer, transaction::Transaction,
};

use crate::accounts_config::{DefaultAccounts, InitializedAccounts};

pub const REFRESH_INCOME_INTERVAL: u64 = 300;

/// Generates fixed distribution from slice
#[macro_export]
macro_rules! distribution {
    ($distribuition:expr) => {{
        let mut new_distribuition = DistributionArray::default();
        new_distribuition[..$distribuition.len()].copy_from_slice(&$distribuition);
        new_distribuition
    }};
}

pub struct Config {
    pub rpc_client: RpcClient,
    pub verbose: bool,
    pub owner: Box<dyn Signer>,
    pub fee_payer: Box<dyn Signer>,
    pub network: String,
}

impl Config {
    pub fn get_default_accounts(&self) -> DefaultAccounts {
        DefaultAccounts::load(&format!("default.{}.yaml", self.network)).unwrap()
    }

    pub fn get_initialized_accounts(&self) -> InitializedAccounts {
        InitializedAccounts::load(&format!("accounts.{}.yaml", self.network)).unwrap_or_default()
    }

    pub fn get_account_deserialize<T: AccountDeserialize>(
        &self,
        pubkey: &Pubkey,
    ) -> Result<T, ClientError> {
        let account = self.rpc_client.get_account(pubkey)?;
        let mut data_ref = &account.data[..];
        let res = T::try_deserialize(&mut data_ref).unwrap();

        Ok(res)
    }

    pub fn get_account_unpack<T: Pack + IsInitialized>(
        &self,
        pubkey: &Pubkey,
    ) -> Result<T, ClientError> {
        let account = self.rpc_client.get_account(pubkey)?;
        let res = T::unpack(&account.data).unwrap();

        Ok(res)
    }

    pub fn sign_and_send_and_confirm_transaction(
        &self,
        mut tx: Transaction,
        signers: Vec<&dyn Signer>,
    ) -> Result<Signature, ClientError> {
        let recent_blockhash = self.rpc_client.get_latest_blockhash()?;

        tx.try_sign(&signers, recent_blockhash)?;

        let signature = self
            .rpc_client
            .send_and_confirm_transaction_with_spinner(&tx)?;

        Ok(signature)
    }
}

pub fn get_program_accounts(
    config: &Config,
    program_id: &Pubkey,
    account_type: u8,
    pubkey: &Pubkey,
) -> Result<Vec<(Pubkey, Account)>, ClientError> {
    config.rpc_client.get_program_accounts_with_config(
        program_id,
        RpcProgramAccountsConfig {
            filters: Some(vec![
                // Account type
                RpcFilterType::Memcmp(Memcmp {
                    offset: 0,
                    bytes: MemcmpEncodedBytes::Base58(bs58::encode([account_type]).into_string()),
                    encoding: Some(MemcmpEncoding::Binary),
                }),
                // Account parent
                RpcFilterType::Memcmp(Memcmp {
                    offset: 1,
                    bytes: MemcmpEncodedBytes::Base58(pubkey.to_string()),
                    encoding: Some(MemcmpEncoding::Binary),
                }),
            ]),
            account_config: RpcAccountInfoConfig {
                encoding: Some(UiAccountEncoding::Base64Zstd),
                ..RpcAccountInfoConfig::default()
            },
            ..RpcProgramAccountsConfig::default()
        },
    )
}

pub fn spl_create_associated_token_account(
    config: &Config,
    wallet: &Pubkey,
    mint: &Pubkey,
) -> Result<Pubkey, ClientError> {
    let associated_token_address =
        spl_associated_token_account::get_associated_token_address(wallet, mint);

    let account_info = config
        .rpc_client
        .get_account_with_commitment(&associated_token_address, config.rpc_client.commitment())?
        .value;
    if account_info.is_some() {
        return Ok(associated_token_address);
    }

    let tx = Transaction::new_with_payer(
        &[
            spl_associated_token_account::create_associated_token_account(
                &config.fee_payer.pubkey(),
                wallet,
                mint,
            ),
        ],
        Some(&config.fee_payer.pubkey()),
    );

    config.sign_and_send_and_confirm_transaction(tx, vec![config.fee_payer.as_ref()])?;

    Ok(associated_token_address)
}

pub fn spl_token_transfer(
    config: &Config,
    source_pubkey: &Pubkey,
    destination_pubkey: &Pubkey,
    amount: u64,
) -> Result<(), ClientError> {
    let tx = Transaction::new_with_payer(
        &[spl_token::instruction::transfer(
            &spl_token::id(),
            source_pubkey,
            destination_pubkey,
            &config.fee_payer.pubkey(),
            &[],
            amount,
        )
        .unwrap()],
        Some(&config.fee_payer.pubkey()),
    );

    config.sign_and_send_and_confirm_transaction(tx, vec![config.fee_payer.as_ref()])?;

    Ok(())
}

#[allow(clippy::type_complexity)]
pub fn get_asset_maps(
    default_accounts: DefaultAccounts,
) -> (
    HashMap<String, Pubkey>,
    HashMap<String, Vec<Option<Pubkey>>>,
) {
    let mint_map = HashMap::from([
        ("SOL".to_string(), default_accounts.sol_mint),
        ("USDC".to_string(), default_accounts.usdc_mint),
        ("USDT".to_string(), default_accounts.usdt_mint),
        ("mSOL".to_string(), default_accounts.msol_mint),
        ("stSOL".to_string(), default_accounts.stsol_mint),
        ("soBTC".to_string(), default_accounts.sobtc_mint),
        ("ETHw".to_string(), default_accounts.ethw_mint),
        ("USTw".to_string(), default_accounts.ustw_mint),
        ("FTTw".to_string(), default_accounts.fttw_mint),
        ("RAY".to_string(), default_accounts.ray_mint),
        ("SRM".to_string(), default_accounts.srm_mint),
    ]);

    let collateral_mint_map = HashMap::from([
        ("SOL".to_string(), default_accounts.sol_collateral),
        ("USDC".to_string(), default_accounts.usdc_collateral),
        ("USDT".to_string(), default_accounts.usdt_collateral),
        ("mSOL".to_string(), default_accounts.msol_collateral),
        ("stSOL".to_string(), default_accounts.stsol_collateral),
        ("soBTC".to_string(), default_accounts.sobtc_collateral),
        ("ETHw".to_string(), default_accounts.ethw_collateral),
        ("USTw".to_string(), default_accounts.ustw_collateral),
        ("FTTw".to_string(), default_accounts.fttw_collateral),
        ("RAY".to_string(), default_accounts.ray_collateral),
        ("SRM".to_string(), default_accounts.srm_collateral),
    ]);

    (mint_map, collateral_mint_map)
}
