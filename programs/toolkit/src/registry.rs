use solana_client::client_error::ClientError;
use solana_program::pubkey::Pubkey;
use solana_sdk::{
    signature::{write_keypair_file, Keypair},
    signer::Signer,
    transaction::Transaction,
};

use everlend_registry::instructions::{UpdateRegistryData, UpdateRegistryMarketsData};

use crate::utils::*;

pub fn init(config: &Config, registry_keypair: Option<Keypair>) -> Result<Pubkey, ClientError> {
    let registry_keypair = registry_keypair.unwrap_or_else(Keypair::new);

    println!("Registry: {}", registry_keypair.pubkey());

    let tx = Transaction::new_with_payer(
        &[everlend_registry::instruction::init(
            &everlend_registry::id(),
            &registry_keypair.pubkey(),
            &config.fee_payer.pubkey(),
        )],
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

pub fn update_registry(
    config: &Config,
    registry_pubkey: &Pubkey,
    data: UpdateRegistryData,
) -> Result<(), ClientError> {
    let tx = Transaction::new_with_payer(
        &[everlend_registry::instruction::update_registry(
            &everlend_registry::id(),
            registry_pubkey,
            &config.fee_payer.pubkey(),
            data,
        )],
        Some(&config.fee_payer.pubkey()),
    );

    config.sign_and_send_and_confirm_transaction(tx, vec![config.fee_payer.as_ref()])?;

    Ok(())
}

pub fn update_registry_markets(
    config: &Config,
    registry_pubkey: &Pubkey,
    data: UpdateRegistryMarketsData,
) -> Result<(), ClientError> {
    let tx = Transaction::new_with_payer(
        &[everlend_registry::instruction::update_registry_markets(
            &everlend_registry::id(),
            registry_pubkey,
            &config.fee_payer.pubkey(),
            data,
        )],
        Some(&config.fee_payer.pubkey()),
    );

    config.sign_and_send_and_confirm_transaction(tx, vec![config.fee_payer.as_ref()])?;

    Ok(())
}

pub fn update_manager(
    config: &Config,
    registry: &Pubkey,
    manager: &Keypair,
    new_manager: &Keypair,
) -> Result<(), ClientError> {
    let tx = Transaction::new_with_payer(
        &[everlend_registry::instruction::update_manager(
            &everlend_registry::id(),
            registry,
            &manager.pubkey(),
            &new_manager.pubkey(),
        )],
        Some(&config.fee_payer.pubkey()),
    );

    config.sign_and_send_and_confirm_transaction(
        tx,
        vec![config.fee_payer.as_ref(), manager, new_manager],
    )?;

    Ok(())
}
