use solana_account_decoder::UiAccountEncoding;
use solana_client::{
    client_error::ClientError,
    rpc_client::RpcClient,
    rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
    rpc_filter::{Memcmp, MemcmpEncodedBytes, MemcmpEncoding, RpcFilterType},
};
use solana_program::pubkey::Pubkey;
use solana_sdk::{
    account::Account, signature::Signature, signer::Signer, transaction::Transaction,
};

use crate::accounts_config::{DefaultAccounts, InitializedAccounts};

pub const REFRESH_INCOME_INTERVAL: u64 = 300;

pub struct Config {
    pub rpc_client: RpcClient,
    pub verbose: bool,
    pub owner: Box<dyn Signer>,
    pub fee_payer: Box<dyn Signer>,
    pub network: String,
}

pub fn get_default_accounts(config: &Config) -> DefaultAccounts {
    DefaultAccounts::load(&format!("default.{}.yaml", config.network)).unwrap_or_default()
}

pub fn get_initialized_accounts(config: &Config) -> InitializedAccounts {
    InitializedAccounts::load(&format!("accounts.{}.yaml", config.network)).unwrap_or_default()
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

pub fn sign_and_send_and_confirm_transaction(
    config: &Config,
    mut tx: Transaction,
    signers: Vec<&dyn Signer>,
) -> Result<Signature, ClientError> {
    let recent_blockhash = config.rpc_client.get_latest_blockhash()?;

    tx.try_sign(&signers, recent_blockhash)?;

    let signature = config
        .rpc_client
        .send_and_confirm_transaction_with_spinner(&tx)?;

    Ok(signature)
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

    sign_and_send_and_confirm_transaction(config, tx, vec![config.fee_payer.as_ref()])?;

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

    sign_and_send_and_confirm_transaction(config, tx, vec![config.fee_payer.as_ref()])?;

    Ok(())
}
