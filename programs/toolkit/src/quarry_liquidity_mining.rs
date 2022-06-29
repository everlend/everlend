use anchor_lang::{prelude::AccountMeta, InstructionData};
use quarry_mine::instruction::CreateMinerV2;
use solana_client::client_error::ClientError;
use solana_program::{
    instruction::Instruction, program_pack::Pack, pubkey::Pubkey, system_instruction,
    system_program,
};
use solana_sdk::{signature::Keypair, signer::Signer, transaction::Transaction};

use crate::utils::Config;

pub fn init_mining_accounts(config: &Config, miner_vault: &Keypair) -> Result<(), ClientError> {
    let default_accounts = config.get_default_accounts();
    let authority = config.fee_payer.pubkey();
    let quarry = default_accounts.quarry_quarry;

    let (miner, _) = Pubkey::find_program_address(
        &[
            "Miner".as_bytes(),
            &quarry.to_bytes(),
            &authority.to_bytes(),
        ],
        &default_accounts.quarry_mine_program_id,
    );
    println!("miner {}", miner);
    let miner_len = quarry_mine::Miner::LEN + 8;
    let token_account_rent = config
        .rpc_client
        .get_minimum_balance_for_rent_exemption(spl_token::state::Account::LEN as usize)?;
    let create_vault_instruction = system_instruction::create_account(
        &config.fee_payer.pubkey(),
        &miner_vault.pubkey(),
        token_account_rent,
        spl_token::state::Account::LEN as u64,
        &spl_token::id(),
    );
    let init_vault_instruction = spl_token::instruction::initialize_account(
        &spl_token::id(),
        &miner_vault.pubkey(),
        &default_accounts.quarry_token_mint,
        &miner,
    )
    .unwrap();
    let create_miner_instruction = Instruction {
        program_id: default_accounts.quarry_mine_program_id,
        accounts: vec![
            AccountMeta::new_readonly(authority, true),
            AccountMeta::new(miner, false),
            AccountMeta::new(quarry, false),
            AccountMeta::new_readonly(default_accounts.quarry_rewarder, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new(config.fee_payer.pubkey(), false),
            AccountMeta::new_readonly(default_accounts.quarry_token_mint, false),
            AccountMeta::new_readonly(miner_vault.pubkey(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: CreateMinerV2.data(),
    };
    let transaction = Transaction::new_with_payer(
        &[
            create_vault_instruction,
            init_vault_instruction,
            create_miner_instruction,
        ],
        Some(&config.fee_payer.pubkey()),
    );
    config.sign_and_send_and_confirm_transaction(
        transaction,
        vec![config.fee_payer.as_ref(), miner_vault],
    )?;
    Ok(())
}
