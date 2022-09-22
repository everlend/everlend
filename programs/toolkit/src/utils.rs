use anchor_lang::AccountDeserialize;
use clap::Arg;
use reqwest::header::{HeaderMap, CONTENT_TYPE};
use serde_json::Value;
use solana_account_decoder::UiAccountEncoding;
use solana_clap_utils::input_validators::{is_amount, is_keypair, is_pubkey};
use solana_client::{
    client_error::ClientError,
    rpc_client::RpcClient,
    rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
    rpc_filter::{Memcmp, MemcmpEncodedBytes, MemcmpEncoding, RpcFilterType},
};
use solana_program::program_pack::{IsInitialized, Pack};
use solana_program::pubkey::Pubkey;
use solana_program::system_instruction;
use solana_sdk::signature::Keypair;
use solana_sdk::{
    account::Account, signature::Signature, signer::Signer, transaction::Transaction,
};
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::{thread, time};

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
    pub owner: Box<dyn Signer>,
    pub fee_payer: Box<dyn Signer>,
    pub network: String,
    pub accounts_path: String,
}

impl Config {
    pub fn get_default_accounts(&self) -> DefaultAccounts {
        DefaultAccounts::load(&format!("default.{}.yaml", self.network)).unwrap()
    }

    pub fn get_initialized_accounts(&self) -> InitializedAccounts {
        InitializedAccounts::load(&format!("accounts.{}.yaml", self.network)).unwrap()
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

pub fn delay(milis: u64) {
    println!("Waiting {} milliseconds...", milis);
    thread::sleep(time::Duration::from_millis(milis))
}

pub fn init_token_account(
    config: &Config,
    account: &Keypair,
    token_mint: &Pubkey,
) -> Result<(), ClientError> {
    let rent = config
        .rpc_client
        .get_minimum_balance_for_rent_exemption(spl_token::state::Account::LEN as usize)?;
    let create_account_instruction = system_instruction::create_account(
        &config.fee_payer.pubkey(),
        &account.pubkey(),
        rent,
        spl_token::state::Account::LEN as u64,
        &spl_token::id(),
    );
    let init_account_instruction = spl_token::instruction::initialize_account(
        &spl_token::id(),
        &account.pubkey(),
        token_mint,
        &config.fee_payer.pubkey(),
    )
    .unwrap();
    let transaction = Transaction::new_with_payer(
        &[create_account_instruction, init_account_instruction],
        Some(&config.fee_payer.pubkey()),
    );
    config.sign_and_send_and_confirm_transaction(
        transaction,
        vec![config.fee_payer.as_ref(), account],
    )?;
    Ok(())
}

pub fn arg_keypair(name: &str, required: bool) -> Arg {
    Arg::with_name(name)
        .long(name)
        .validator(is_keypair)
        .value_name("KEYPAIR")
        .takes_value(true)
        .required(required)
        .help("Keypair [default: new keypair]")
}

pub fn arg_pubkey(name: &str, required: bool) -> Arg {
    Arg::with_name(name)
        .long(name)
        .validator(is_pubkey)
        .value_name("ADDRESS")
        .takes_value(true)
        .required(required)
        .help("Pubkey")
}

pub fn arg_amount(name: &str, required: bool) -> Arg {
    Arg::with_name(name)
        .long(name)
        .validator(is_amount)
        .value_name("NUMBER")
        .takes_value(true)
        .required(required)
}

pub fn arg_multiple(name: &str, required: bool) -> Arg {
    Arg::with_name(name)
        .multiple(true)
        .long(name)
        .required(required)
        .min_values(1)
        .takes_value(true)
}

pub fn arg_path(name: &str, required: bool) -> Arg {
    Arg::with_name(name)
        .long(name)
        .value_name("PATH")
        .takes_value(true)
        .required(required)
}

pub fn arg(name: &str, required: bool) -> Arg {
    Arg::with_name(name)
        .long(name)
        .takes_value(true)
        .required(required)
}

pub fn download_account(pubkey: &Pubkey, mm_name: &str, account_name: &str) {
    let client = reqwest::blocking::Client::new();
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());
    let res = client
        .post("https://api.devnet.solana.com")
        .headers(headers)
        .body(format!(
            "
        {{
            \"jsonrpc\": \"2.0\",
            \"id\": 1,
            \"method\": \"getAccountInfo\",
            \"params\": [
                \"{}\",
                {{
                \"encoding\": \"base64\"
                }}
            ]
        }}
        ",
            pubkey
        ))
        .send()
        .expect("failed to get response")
        .text()
        .expect("failed to get payload");
    let json: Value = serde_json::from_str(&res).unwrap();
    let data = &json["result"]["value"]["data"][0];
    let string = data.as_str().unwrap();
    let bytes = base64::decode(string).unwrap();
    let mut file = File::create(format!(
        "../tests/tests/fixtures/{}/{}.bin",
        mm_name, account_name
    ))
    .unwrap();
    file.write_all(bytes.as_slice()).unwrap();
    println!("{} {}", account_name, pubkey);
}
