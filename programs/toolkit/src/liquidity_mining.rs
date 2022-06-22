use everlend_utils::find_program_address;
use larix_lending::instruction::LendingInstruction;
use solana_client::client_error::ClientError;
use solana_program::{
    instruction::{AccountMeta, Instruction},
    program_pack::Pack,
    system_instruction,
};
use solana_sdk::{signature::Keypair, signer::Signer, transaction::Transaction};

use everlend_depositor::{instruction::InitMiningAccountsPubkeys, state::MiningType};
use solana_program::pubkey::Pubkey;

use crate::{accounts_config::InitializedAccounts, utils::*};

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

pub fn init_mining_accounts_larix(
    config: &Config,
    mining_account: &Keypair,
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
        vec![config.fee_payer.as_ref(), mining_account],
    )?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn deposit_larix(
    config: &Config,
    accounts_path: &str,
    liquidity_amount: u64,
    source: &Pubkey,
    destination_collateral: &Pubkey,
) -> Result<(), ClientError> {
    let default_accounts = config.get_default_accounts();
    let initialized_accounts = InitializedAccounts::load(accounts_path).unwrap();
    let sol = initialized_accounts.token_accounts.get("SOL").unwrap();
    let lending_market = default_accounts.larix_lending_market;
    let (lending_market_authority, _) =
        find_program_address(&default_accounts.larix_program_id, &lending_market);
    let rent = config
        .rpc_client
        .get_minimum_balance_for_rent_exemption(spl_token::state::Account::LEN as usize)?;

    let create_account_instruction = system_instruction::create_account(
        &spl_token::id(),
        destination_collateral,
        rent,
        spl_token::state::Account::LEN as u64,
        &default_accounts.larix_program_id,
    );
    let init_account_instruction = spl_token::instruction::initialize_account(
        &spl_token::id(),
        destination_collateral,
        &sol.mint,
        &config.fee_payer.pubkey(),
    )
    .unwrap();

    let deposit_mining_instruction = Instruction {
        program_id: default_accounts.larix_program_id,
        accounts: vec![
            AccountMeta::new(*source, false),
            AccountMeta::new(*destination_collateral, false),
            AccountMeta::new(default_accounts.larix_reserve_sol, false),
            AccountMeta::new_readonly(default_accounts.larix_reserve_sol_supply, false),
            AccountMeta::new_readonly(sol.mint, false),
            AccountMeta::new_readonly(lending_market, false),
            AccountMeta::new_readonly(lending_market_authority, false),
            AccountMeta::new_readonly(config.fee_payer.pubkey(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: LendingInstruction::DepositReserveLiquidity { liquidity_amount }.pack(),
    };
    let tx = Transaction::new_with_payer(
        &[
            create_account_instruction,
            init_account_instruction,
            deposit_mining_instruction,
        ],
        Some(&config.fee_payer.pubkey()),
    );

    config.sign_and_send_and_confirm_transaction(tx, vec![config.fee_payer.as_ref()])?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn deposit_mining_larix(
    config: &Config,
    accounts_path: &str,
    amount: u64,
    source: &Pubkey,
    mining: &Pubkey,
    collateral_transit: &Pubkey,
) -> Result<(), ClientError> {
    let default_accounts = config.get_default_accounts();
    let initialized_accounts = InitializedAccounts::load(accounts_path).unwrap();
    let token_account = initialized_accounts.token_accounts.get("SOL").unwrap();
    println!("token account mint {:?}", token_account.mint);
    let deposit_mining_instruction = Instruction {
        program_id: default_accounts.larix_program_id,
        accounts: vec![
            AccountMeta::new(*source, false),
            AccountMeta::new(*collateral_transit, false),
            AccountMeta::new(*mining, false),
            AccountMeta::new_readonly(default_accounts.larix_reserve_sol, false),
            AccountMeta::new_readonly(default_accounts.larix_lending_market, false),
            AccountMeta::new_readonly(config.fee_payer.pubkey(), false),
            AccountMeta::new_readonly(config.fee_payer.pubkey(), true),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: LendingInstruction::DepositMining { amount }.pack(),
    };
    let tx = Transaction::new_with_payer(
        &[deposit_mining_instruction],
        Some(&config.fee_payer.pubkey()),
    );

    config.sign_and_send_and_confirm_transaction(tx, vec![config.fee_payer.as_ref()])?;

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
