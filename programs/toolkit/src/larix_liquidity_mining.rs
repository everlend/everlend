use std::str::FromStr;

use everlend_utils::find_program_address;
use larix_lending::instruction::LendingInstruction;
use solana_client::client_error::ClientError;
use solana_program::{
    instruction::{AccountMeta, Instruction},
    program_pack::Pack,
    system_instruction,
};
use solana_sdk::{signature::Keypair, signer::Signer, transaction::Transaction};

use solana_program::pubkey::Pubkey;

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
    let transaction = Transaction::new_with_payer(
        &[create_account_instruction],
        Some(&config.fee_payer.pubkey()),
    );
    config.sign_and_send_and_confirm_transaction(
        transaction,
        vec![config.fee_payer.as_ref(), mining_account],
    )?;
    Ok(())
}

pub fn init_mining_accounts(config: &Config, mining_account: &Keypair) -> Result<(), ClientError> {
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
    let transaction = Transaction::new_with_payer(
        &[create_account_instruction, init_mining_instruction],
        Some(&config.fee_payer.pubkey()),
    );
    config.sign_and_send_and_confirm_transaction(
        transaction,
        vec![config.fee_payer.as_ref(), mining_account],
    )?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn deposit_liquidity(
    config: &Config,
    liquidity_amount: u64,
    source: &Pubkey,
    collateral_transit: &Keypair,
) -> Result<(), ClientError> {
    let default_accounts = config.get_default_accounts();
    let collateral_mint = default_accounts.sol_collateral.get(1).unwrap().unwrap();
    let lending_market = default_accounts.larix_lending_market;
    let (lending_market_authority, _) =
        find_program_address(&default_accounts.larix_program_id, &lending_market);
    let rent = config
        .rpc_client
        .get_minimum_balance_for_rent_exemption(spl_token::state::Account::LEN as usize)?;
    let create_account_instruction = system_instruction::create_account(
        &config.fee_payer.pubkey(),
        &collateral_transit.pubkey(),
        rent,
        spl_token::state::Account::LEN as u64,
        &spl_token::id(),
    );
    let init_account_instruction = spl_token::instruction::initialize_account(
        &spl_token::id(),
        &collateral_transit.pubkey(),
        &collateral_mint,
        &config.fee_payer.pubkey(),
    )
    .unwrap();
    let refresh_instruction = Instruction {
        program_id: default_accounts.larix_program_id,
        accounts: vec![
            AccountMeta::new(default_accounts.larix_reserve_sol, false),
            AccountMeta::new_readonly(default_accounts.sol_oracle, false),
        ],
        data: LendingInstruction::RefreshReserves {}.pack(),
    };
    let deposit_instruction = Instruction {
        program_id: default_accounts.larix_program_id,
        accounts: vec![
            AccountMeta::new(*source, false),
            AccountMeta::new(collateral_transit.pubkey(), false),
            AccountMeta::new(default_accounts.larix_reserve_sol, false),
            AccountMeta::new(collateral_mint, false),
            AccountMeta::new(default_accounts.larix_reserve_sol_supply, false),
            AccountMeta::new_readonly(lending_market, false),
            AccountMeta::new_readonly(lending_market_authority, false),
            AccountMeta::new_readonly(config.fee_payer.pubkey(), true),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: LendingInstruction::DepositReserveLiquidity { liquidity_amount }.pack(),
    };
    let transaction = Transaction::new_with_payer(
        &[
            create_account_instruction,
            init_account_instruction,
            refresh_instruction,
            deposit_instruction,
        ],
        Some(&config.fee_payer.pubkey()),
    );
    config.sign_and_send_and_confirm_transaction(
        transaction,
        vec![config.fee_payer.as_ref(), collateral_transit],
    )?;
    let balance = config
        .rpc_client
        .get_token_account_balance(&collateral_transit.pubkey())
        .unwrap();
    println!("balance {:?}", balance);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn deposit_collateral(
    config: &Config,
    amount: u64,
    mining: &Pubkey,
    collateral_transit: &Pubkey,
) -> Result<(), ClientError> {
    let default_accounts = config.get_default_accounts();
    let refresh_instruction = Instruction {
        program_id: default_accounts.larix_program_id,
        accounts: vec![
            AccountMeta::new(default_accounts.larix_reserve_sol, false),
            AccountMeta::new_readonly(default_accounts.sol_oracle, false),
        ],
        data: LendingInstruction::RefreshReserves {}.pack(),
    };
    let deposit_mining_instruction = Instruction {
        program_id: default_accounts.larix_program_id,
        accounts: vec![
            AccountMeta::new(*collateral_transit, false),
            AccountMeta::new(default_accounts.larix_uncollateralized_ltoken_supply, false),
            AccountMeta::new(*mining, false),
            AccountMeta::new_readonly(default_accounts.larix_reserve_sol, false),
            AccountMeta::new_readonly(default_accounts.larix_lending_market, false),
            AccountMeta::new_readonly(config.fee_payer.pubkey(), false),
            AccountMeta::new_readonly(config.fee_payer.pubkey(), true),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: LendingInstruction::DepositMining { amount }.pack(),
    };
    let transaction = Transaction::new_with_payer(
        &[refresh_instruction, deposit_mining_instruction],
        Some(&config.fee_payer.pubkey()),
    );
    config.sign_and_send_and_confirm_transaction(transaction, vec![config.fee_payer.as_ref()])?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn withdraw_collateral(
    config: &Config,
    amount: u64,
    destination: &Keypair,
    mining: &Pubkey,
) -> Result<(), ClientError> {
    let default_accounts = config.get_default_accounts();
    let collateral_mint = default_accounts.sol_collateral.get(1).unwrap().unwrap();
    let lending_market = default_accounts.larix_lending_market;
    let (lending_market_authority, _) =
        find_program_address(&default_accounts.larix_program_id, &lending_market);
    let rent = config
        .rpc_client
        .get_minimum_balance_for_rent_exemption(spl_token::state::Account::LEN as usize)?;
    let create_account_instruction = system_instruction::create_account(
        &config.fee_payer.pubkey(),
        &destination.pubkey(),
        rent,
        spl_token::state::Account::LEN as u64,
        &spl_token::id(),
    );
    let init_account_instruction = spl_token::instruction::initialize_account(
        &spl_token::id(),
        &destination.pubkey(),
        &collateral_mint,
        &config.fee_payer.pubkey(),
    )
    .unwrap();
    let refresh_instruction = Instruction {
        program_id: default_accounts.larix_program_id,
        accounts: vec![
            AccountMeta::new(default_accounts.larix_reserve_sol, false),
            AccountMeta::new_readonly(default_accounts.sol_oracle, false),
        ],
        data: LendingInstruction::RefreshReserves {}.pack(),
    };
    let clock_id = Pubkey::from_str("SysvarC1ock11111111111111111111111111111111").unwrap();
    let deposit_mining_instruction = Instruction {
        program_id: default_accounts.larix_program_id,
        accounts: vec![
            AccountMeta::new(default_accounts.larix_uncollateralized_ltoken_supply, false),
            AccountMeta::new(destination.pubkey(), false),
            AccountMeta::new(*mining, false),
            AccountMeta::new_readonly(default_accounts.larix_reserve_sol, false),
            AccountMeta::new_readonly(lending_market, false),
            AccountMeta::new_readonly(lending_market_authority, false),
            AccountMeta::new_readonly(config.fee_payer.pubkey(), false),
            AccountMeta::new_readonly(clock_id, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: LendingInstruction::WithdrawMining { amount }.pack(),
    };
    let transaction = Transaction::new_with_payer(
        &[
            create_account_instruction,
            init_account_instruction,
            refresh_instruction,
            deposit_mining_instruction,
        ],
        Some(&config.fee_payer.pubkey()),
    );
    config.sign_and_send_and_confirm_transaction(
        transaction,
        vec![config.fee_payer.as_ref(), destination],
    )?;
    let balance = config
        .rpc_client
        .get_token_account_balance(&destination.pubkey())
        .unwrap();
    println!("balance {:?}", balance);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn claim_mining(
    config: &Config,
    destination: &Keypair,
    mining: &Pubkey,
) -> Result<(), ClientError> {
    let default_accounts = config.get_default_accounts();
    let lending_market = default_accounts.larix_lending_market;
    let (lending_market_authority, _) =
        find_program_address(&default_accounts.larix_program_id, &lending_market);
    let rent = config
        .rpc_client
        .get_minimum_balance_for_rent_exemption(spl_token::state::Account::LEN as usize)?;
    let create_account_instruction = system_instruction::create_account(
        &config.fee_payer.pubkey(),
        &destination.pubkey(),
        rent,
        spl_token::state::Account::LEN as u64,
        &spl_token::id(),
    );
    let init_account_instruction = spl_token::instruction::initialize_account(
        &spl_token::id(),
        &destination.pubkey(),
        &default_accounts.larix_ltoken_mint,
        &config.fee_payer.pubkey(),
    )
    .unwrap();
    let refresh_instruction = Instruction {
        program_id: default_accounts.larix_program_id,
        accounts: vec![
            AccountMeta::new(default_accounts.larix_reserve_sol, false),
            AccountMeta::new_readonly(default_accounts.sol_oracle, false),
        ],
        data: LendingInstruction::RefreshReserves {}.pack(),
    };
    let claim_instruction = Instruction {
        program_id: default_accounts.larix_program_id,
        accounts: vec![
            AccountMeta::new(*mining, false),
            AccountMeta::new(default_accounts.larix_mining_supply, false),
            AccountMeta::new(destination.pubkey(), false),
            AccountMeta::new_readonly(config.fee_payer.pubkey(), true),
            AccountMeta::new_readonly(lending_market, false),
            AccountMeta::new_readonly(lending_market_authority, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(default_accounts.larix_reserve_sol, false),
        ],
        data: LendingInstruction::ClaimMiningMine.pack(),
    };
    let transaction = Transaction::new_with_payer(
        &[
            create_account_instruction,
            init_account_instruction,
            refresh_instruction,
            claim_instruction,
        ],
        Some(&config.fee_payer.pubkey()),
    );
    config.sign_and_send_and_confirm_transaction(
        transaction,
        vec![config.fee_payer.as_ref(), destination],
    )?;
    let balance = config
        .rpc_client
        .get_token_account_balance(&destination.pubkey())
        .unwrap();
    println!("balance {:?}", balance);
    Ok(())
}
