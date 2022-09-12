use solana_client::client_error::ClientError;
use solana_program::{program_pack::Pack, pubkey::Pubkey, system_instruction};
use solana_sdk::{
    signature::{write_keypair_file, Keypair},
    signer::Signer,
    transaction::Transaction,
};

use everlend_registry::{
    find_config_program_address,
    state::{Registry, RegistryPrograms, RegistryRootAccounts, RegistrySettings},
};

use crate::utils::*;

pub fn init_registry(
    config: &Config,
    registry_keypair: Option<Keypair>,
) -> Result<Pubkey, ClientError> {
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
    programs: RegistryPrograms,
    roots: RegistryRootAccounts,
    settings: RegistrySettings,
) -> Result<Pubkey, ClientError> {
    let tx = Transaction::new_with_payer(
        &[everlend_registry::instruction::set_registry_config(
            &everlend_registry::id(),
            registry_pubkey,
            &config.fee_payer.pubkey(),
            programs,
            roots,
            settings,
        )],
        Some(&config.fee_payer.pubkey()),
    );

    config.sign_and_send_and_confirm_transaction(tx, vec![config.fee_payer.as_ref()])?;

    let (registry_config_pubkey, _) =
        find_config_program_address(&everlend_registry::id(), registry_pubkey);

    Ok(registry_config_pubkey)
}

pub fn close_registry_config(config: &Config, registry_pubkey: &Pubkey) -> Result<(), ClientError> {
    println!("Sending CloseRegistryConfig itx ...");
    let tx = Transaction::new_with_payer(
        &[everlend_registry::instruction::close_registry_config(
            &everlend_registry::id(),
            registry_pubkey,
            &config.fee_payer.pubkey(),
        )],
        Some(&config.fee_payer.pubkey()),
    );

    config.sign_and_send_and_confirm_transaction(tx, vec![config.fee_payer.as_ref()])?;

    let (config_registry_pubkey, _) =
        find_config_program_address(&everlend_registry::id(), registry_pubkey);

    while config
        .rpc_client
        .get_account(&config_registry_pubkey)
        .is_ok()
    {
        delay(500);
    }

    println!("RegistryConfig account closed.");

    Ok(())
}

pub fn registry_update_manager(
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
