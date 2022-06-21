use larix_lending::instruction::LendingInstruction;
use solana_client::client_error::ClientError;
use solana_program::{
    instruction::{AccountMeta, Instruction},
    system_instruction,
};
use solana_sdk::{signature::Keypair, signer::Signer, transaction::Transaction};

use everlend_depositor::{instruction::InitMiningAccountsPubkeys, state::MiningType};

use crate::utils::*;

const LARIX_MINING_SIZE: u64 = 1 + 32 + 32 + 1 + 16 + 560;

pub fn create_mining_account(config: &Config, mining_account: &Keypair) -> Result<(), ClientError> {
    let default_accounts = config.get_default_accounts();
    let rent = config
        .rpc_client
        .get_minimum_balance_for_rent_exemption(LARIX_MINING_SIZE as usize)?;
    let create_account_instruction = system_instruction::create_account(
        &config.fee_payer.pubkey(),
        &mining_account.pubkey(),
        rent,
        LARIX_MINING_SIZE,
        &default_accounts.larix_program_id,
    );
    let tx = Transaction::new_with_payer(
        &[create_account_instruction],
        Some(&config.fee_payer.pubkey()),
    );

    config.sign_and_send_and_confirm_transaction(
        tx,
        vec![config.fee_payer.as_ref(), mining_account],
    )?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn init_mining_accounts_larix(
    config: &Config,
    mining_account: Keypair,
) -> Result<(), ClientError> {
    let default_accounts = config.get_default_accounts();
    let rent = config
        .rpc_client
        .get_minimum_balance_for_rent_exemption(LARIX_MINING_SIZE as usize)?;
    let create_account_instruction = system_instruction::create_account(
        &config.fee_payer.pubkey(),
        &mining_account.pubkey(),
        rent,
        LARIX_MINING_SIZE,
        &default_accounts.larix_program_id,
    );
    let init_mining_instruction = Instruction {
        program_id: default_accounts.larix_program_id,
        accounts: vec![
            AccountMeta::new(mining_account.pubkey(), false),
            AccountMeta::new_readonly(config.fee_payer.pubkey(), true),
            AccountMeta::new_readonly(default_accounts.larix_lending_market, false),
        ],
        data: LendingInstruction::InitMining.pack(),
    };
    let tx = Transaction::new_with_payer(
        &[create_account_instruction, init_mining_instruction],
        Some(&config.fee_payer.pubkey()),
    );

    config.sign_and_send_and_confirm_transaction(
        tx,
        vec![config.fee_payer.as_ref(), &mining_account],
    )?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn init_larix_mining_with_depositor(
    config: &Config,
    pubkeys: InitMiningAccountsPubkeys,
    mining_account: &Keypair,
    mining_type: MiningType,
) -> Result<(), ClientError> {
    create_mining_account(config, mining_account)?;
    let tx = Transaction::new_with_payer(
        &[everlend_depositor::instruction::init_mining_accounts(
            &everlend_depositor::id(),
            pubkeys,
            mining_type,
        )],
        Some(&config.fee_payer.pubkey()),
    );
    config.sign_and_send_and_confirm_transaction(tx, vec![config.fee_payer.as_ref()])?;

    Ok(())
}
