use solana_program::bpf_loader_upgradeable;
use solana_program::pubkey::Pubkey;

use crate::multisig::{self, get_multisig_program_address};
use crate::utils::Config;

pub async fn command_create_multisig(
    config: &Config,
    owners: Vec<Pubkey>,
    threshold: u64,
) -> anyhow::Result<()> {
    println!("owners = {:#?}", owners);
    println!("threshold = {:?}", threshold);

    let (multisig_pubkey, multisig_pda) =
        multisig::create_multisig(config, None, owners, threshold)?;

    println!("multisig_pubkey = {:?}", multisig_pubkey);
    println!("multisig_pda = {:?}", multisig_pda);

    Ok(())
}

pub async fn command_propose_upgrade(
    config: &Config,
    program_pubkey: &Pubkey,
    buffer_pubkey: &Pubkey,
    spill_pubkey: &Pubkey,
    multisig_pubkey: &Pubkey,
) -> anyhow::Result<()> {
    let default_accounts = config.get_default_accounts();
    let (pda, _) =
        get_multisig_program_address(&default_accounts.multisig_program_id, multisig_pubkey);

    let upgrade_instruction =
        bpf_loader_upgradeable::upgrade(program_pubkey, buffer_pubkey, &pda, spill_pubkey);

    let transaction_pubkey =
        multisig::create_transaction(config, multisig_pubkey, upgrade_instruction)?;

    println!("transaction_pubkey = {:?}", transaction_pubkey);

    Ok(())
}
