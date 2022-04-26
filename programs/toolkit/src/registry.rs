use std::convert::TryInto;

use solana_client::client_error::ClientError;
use solana_program::{program_pack::Pack, pubkey::Pubkey, system_instruction};
use solana_sdk::{
    signature::{write_keypair_file, Keypair},
    signer::Signer,
    transaction::Transaction,
};

use everlend_registry::state::{PoolMarketsConfig, RegistryConfig, TOTAL_DISTRIBUTIONS};
use everlend_registry::{
    find_config_program_address,
    state::{Registry, SetRegistryConfigParams},
};

use crate::utils::*;

pub fn init(config: &Config, registry_keypair: Option<Keypair>) -> Result<Pubkey, ClientError> {
    let registry_keypair = registry_keypair.unwrap_or_else(Keypair::new);

    println!("Registry: {}", registry_keypair.pubkey());

    let balance = config
        .rpc_client
        .get_minimum_balance_for_rent_exemption(Registry::LEN)?;

    let tx = Transaction::new_with_payer(
        &[
            system_instruction::create_account(
                &config.fee_payer.pubkey(),
                &registry_keypair.pubkey(),
                balance,
                Registry::LEN as u64,
                &everlend_registry::id(),
            ),
            everlend_registry::instruction::init(
                &everlend_registry::id(),
                &registry_keypair.pubkey(),
                &config.fee_payer.pubkey(),
            ),
        ],
        Some(&config.fee_payer.pubkey()),
    );

    config.sign_and_send_and_confirm_transaction(
        tx,
        vec![config.fee_payer.as_ref(), &registry_keypair],
    )?;

    write_keypair_file(
        &registry_keypair,
        &format!(".keypairs/{}.json", registry_keypair.pubkey()),
    )
    .unwrap();

    Ok(registry_keypair.pubkey())
}

pub fn set_registry_config(
    config: &Config,
    registry_pubkey: &Pubkey,
    params: SetRegistryConfigParams,
    pool_markets_cfg: PoolMarketsConfig,
) -> Result<Pubkey, ClientError> {
    let tx = Transaction::new_with_payer(
        &[everlend_registry::instruction::set_registry_config(
            &everlend_registry::id(),
            registry_pubkey,
            &config.fee_payer.pubkey(),
            params,
            pool_markets_cfg,
        )],
        Some(&config.fee_payer.pubkey()),
    );

    config.sign_and_send_and_confirm_transaction(tx, vec![config.fee_payer.as_ref()])?;

    let (registry_config_pubkey, _) =
        find_config_program_address(&everlend_registry::id(), registry_pubkey);

    Ok(registry_config_pubkey)
}

pub fn migrate_registry_config(config: &Config) -> Result<(), anyhow::Error> {
    let accounts = config.get_initialized_accounts();

    let mut ulp_pool_markets = accounts.mm_pool_markets.clone();

    ulp_pool_markets.extend_from_slice(
        vec![Pubkey::default(); TOTAL_DISTRIBUTIONS - ulp_pool_markets.len()].as_slice(),
    );

    let pool_markets_cfg = PoolMarketsConfig {
        general_pool_market: accounts.general_pool_market,
        income_pool_market: accounts.income_pool_market,
        ulp_pool_markets: ulp_pool_markets.try_into().unwrap(),
    };

    println!("Sending migration itx ...");
    let tx = Transaction::new_with_payer(
        &[everlend_registry::instruction::migrate_registry_config(
            &everlend_registry::id(),
            &accounts.registry,
            &config.fee_payer.pubkey(),
            pool_markets_cfg,
        )],
        Some(&config.fee_payer.pubkey()),
    );

    config.sign_and_send_and_confirm_transaction(tx, vec![config.fee_payer.as_ref()])?;

    let (new_registry_config, _) =
        find_config_program_address(&everlend_registry::id(), &accounts.registry);

    let new_registry: RegistryConfig = config.get_account_unpack(&new_registry_config)?;

    println!(
        "Migration of RgistryConfig successfully: \n{:?}",
        new_registry
    );

    Ok(())
}
