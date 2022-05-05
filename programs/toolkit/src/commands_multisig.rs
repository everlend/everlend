use anchor_lang::AccountDeserialize;
use solana_program::bpf_loader_upgradeable;
use solana_program::pubkey::Pubkey;

use crate::multisig::{self, get_multisig_program_address, get_transaction_program_accounts};
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

pub async fn command_info_multisig(
    config: &Config,
    multisig_pubkey: &Pubkey,
) -> anyhow::Result<()> {
    let multisig = config.get_account_deserialize::<serum_multisig::Multisig>(multisig_pubkey)?;

    println!("Owners: {:?}", multisig.owners);
    println!("Threshold: {:?}", multisig.threshold);

    println!("Transactions:");
    let txs: Vec<(Pubkey, serum_multisig::Transaction)> =
        get_transaction_program_accounts(config, multisig_pubkey)?
            .into_iter()
            .filter_map(|(address, account)| {
                let mut data_ref = &account.data[..];
                match serum_multisig::Transaction::try_deserialize(&mut data_ref) {
                    Ok(tx) => Some((address, tx)),
                    _ => None,
                }
            })
            .collect();

    for (pubkey, tx) in txs {
        println!("{:?}", pubkey);
        println!("Data: {:?}", tx.data);
        println!("Signers: {:?}", tx.signers);
        println!("Set seqno: {:?}", tx.owner_set_seqno);
        println!("Executed: {:?}", tx.did_execute);
    }

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

pub async fn command_approve(
    config: &Config,
    multisig_pubkey: &Pubkey,
    transaction_pubkey: &Pubkey,
) -> anyhow::Result<()> {
    println!("transaction_pubkey = {:#?}", transaction_pubkey);
    println!("multisig_pubkey = {:?}", multisig_pubkey);

    let signature = multisig::approve(config, multisig_pubkey, transaction_pubkey)?;

    println!("signature = {:?}", signature);

    Ok(())
}

pub async fn command_execute_transaction(
    config: &Config,
    multisig_pubkey: &Pubkey,
    transaction_pubkey: &Pubkey,
) -> anyhow::Result<()> {
    println!("transaction_pubkey = {:#?}", transaction_pubkey);
    println!("multisig_pubkey = {:?}", multisig_pubkey);

    let signature = multisig::execute_transaction(config, multisig_pubkey, transaction_pubkey)?;

    println!("signature = {:?}", signature);

    Ok(())
}
