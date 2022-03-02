use crate::utils::*;
use everlend_registry::{
    find_config_program_address,
    state::{Registry, SetRegistryConfigParams},
};
use solana_client::client_error::ClientError;
use solana_program::{program_pack::Pack, pubkey::Pubkey, system_instruction};
use solana_sdk::{signature::Keypair, signer::Signer, transaction::Transaction};

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

    sign_and_send_and_confirm_transaction(
        config,
        tx,
        vec![config.fee_payer.as_ref(), &registry_keypair],
    )?;

    Ok(registry_keypair.pubkey())
}

pub fn set_registry_config(
    config: &Config,
    registry_pubkey: &Pubkey,
    params: SetRegistryConfigParams,
) -> Result<Pubkey, ClientError> {
    let tx = Transaction::new_with_payer(
        &[everlend_registry::instruction::set_registry_config(
            &everlend_registry::id(),
            registry_pubkey,
            &config.fee_payer.pubkey(),
            params,
        )],
        Some(&config.fee_payer.pubkey()),
    );

    sign_and_send_and_confirm_transaction(config, tx, vec![config.fee_payer.as_ref()])?;

    let (registry_config_pubkey, _) =
        find_config_program_address(&everlend_registry::id(), registry_pubkey);

    Ok(registry_config_pubkey)
}
