use anchor_lang::{prelude::AccountMeta, InstructionData};
use quarry_mine::instruction::{ClaimRewardsV2, CreateMinerV2, StakeTokens};
use solana_client::client_error::ClientError;
use solana_program::{
    instruction::Instruction, program_pack::Pack, pubkey::Pubkey, system_instruction,
    system_program,
};
use solana_sdk::{signature::Keypair, signer::Signer, transaction::Transaction};

use crate::utils::Config;

pub fn create_miner(config: &Config, miner_vault: &Keypair) -> Result<(), ClientError> {
    let default_accounts = config.get_default_accounts();
    let miner = find_miner_address(config);
    println!("miner {}", miner);
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
        &default_accounts.quarry.token_mint,
        &miner,
    )
    .unwrap();
    let create_miner_instruction = Instruction {
        program_id: default_accounts.quarry.mine_program_id,
        accounts: vec![
            AccountMeta::new_readonly(config.fee_payer.pubkey(), true),
            AccountMeta::new(miner, false),
            AccountMeta::new(default_accounts.quarry.quarry, false),
            AccountMeta::new_readonly(default_accounts.quarry.rewarder, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new(config.fee_payer.pubkey(), false),
            AccountMeta::new_readonly(default_accounts.quarry.token_mint, false),
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

pub fn stake_tokens(config: &Config, token: &String, amount: u64) -> Result<(), ClientError> {
    let default_accounts = config.get_default_accounts();
    let mut initialized_accounts = config.get_initialized_accounts();
    let quarry_mining = initialized_accounts.quarry_mining.get_mut(token).unwrap();
    let miner = find_miner_address(config);
    let stake_instruction = Instruction {
        program_id: default_accounts.quarry.mine_program_id,
        accounts: vec![
            AccountMeta::new_readonly(config.fee_payer.pubkey(), true),
            AccountMeta::new(miner, false),
            AccountMeta::new(default_accounts.quarry.quarry, false),
            AccountMeta::new(quarry_mining.miner_vault, false),
            AccountMeta::new(quarry_mining.token_source, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(default_accounts.quarry.rewarder, false),
        ],
        data: StakeTokens { amount }.data(),
    };
    let transaction =
        Transaction::new_with_payer(&[stake_instruction], Some(&config.fee_payer.pubkey()));
    config.sign_and_send_and_confirm_transaction(transaction, vec![config.fee_payer.as_ref()])?;
    let balance = config
        .rpc_client
        .get_token_account_balance(&quarry_mining.token_source)
        .unwrap();
    println!("balance of rewards token account {:?}", balance);
    Ok(())
}

pub fn withdraw_tokens(config: &Config, token: &String, amount: u64) -> Result<(), ClientError> {
    let default_accounts = config.get_default_accounts();
    let mut initialized_accounts = config.get_initialized_accounts();
    let quarry_mining = initialized_accounts.quarry_mining.get_mut(token).unwrap();
    let miner = find_miner_address(config);
    let stake_instruction = Instruction {
        program_id: default_accounts.quarry.mine_program_id,
        accounts: vec![
            AccountMeta::new_readonly(config.fee_payer.pubkey(), true),
            AccountMeta::new(miner, false),
            AccountMeta::new(default_accounts.quarry.quarry, false),
            AccountMeta::new(quarry_mining.miner_vault, false),
            AccountMeta::new(quarry_mining.token_source, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(quarry_mining.token_source, false),
        ],
        data: StakeTokens { amount }.data(),
    };
    let transaction =
        Transaction::new_with_payer(&[stake_instruction], Some(&config.fee_payer.pubkey()));
    config.sign_and_send_and_confirm_transaction(transaction, vec![config.fee_payer.as_ref()])?;
    let balance = config
        .rpc_client
        .get_token_account_balance(&quarry_mining.token_source)
        .unwrap();
    println!("balance of rewards token account {:?}", balance);
    Ok(())
}

pub fn claim_mining_rewards(config: &Config, token: &String) -> Result<(), ClientError> {
    let default_accounts = config.get_default_accounts();
    let mut initialized_accounts = config.get_initialized_accounts();
    let quarry_mining = initialized_accounts.quarry_mining.get_mut(token).unwrap();
    let miner = find_miner_address(config);
    let instruction = Instruction {
        program_id: default_accounts.quarry.mine_program_id,
        accounts: vec![
            AccountMeta::new(default_accounts.quarry.mint_wrapper, false),
            AccountMeta::new_readonly(default_accounts.quarry.mint_wrapper_program, false),
            AccountMeta::new(default_accounts.quarry.minter, false),
            AccountMeta::new(quarry_mining.rewards_token_mint, false),
            AccountMeta::new(quarry_mining.rewards_token_account, false),
            AccountMeta::new(quarry_mining.fee_token_account, false),
            AccountMeta::new(config.fee_payer.pubkey(), false),
            AccountMeta::new(miner, false),
            AccountMeta::new(default_accounts.quarry.quarry, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(default_accounts.quarry.rewarder, false),
        ],
        data: ClaimRewardsV2 {}.data(),
    };
    let transaction = Transaction::new_with_payer(&[instruction], Some(&config.fee_payer.pubkey()));
    config.sign_and_send_and_confirm_transaction(transaction, vec![config.fee_payer.as_ref()])?;
    let balance = config
        .rpc_client
        .get_token_account_balance(&quarry_mining.rewards_token_account)
        .unwrap();
    println!("balance of rewards token account {:?}", balance);
    Ok(())
}

fn find_miner_address(config: &Config) -> Pubkey {
    let default_accounts = config.get_default_accounts();
    let authority = config.fee_payer.pubkey();
    let quarry = default_accounts.quarry.quarry;
    let (miner, _) = Pubkey::find_program_address(
        &[
            "Miner".as_bytes(),
            &quarry.to_bytes(),
            &authority.to_bytes(),
        ],
        &default_accounts.quarry.mine_program_id,
    );
    miner
}
