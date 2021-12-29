//! Instruction types

use crate::find_pool_program_address;
use borsh::{BorshDeserialize, BorshSerialize};
use everlend_utils::find_program_address;
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_program, sysvar,
};

/// Instructions supported by the program
#[derive(Debug, BorshDeserialize, BorshSerialize, PartialEq)]
pub enum IncomePoolsInstruction {
    /// Initializes a new income pool market
    ///
    /// Accounts:
    /// [W] Income pool market - uninitialized
    /// [R] Market manager
    /// [R] General pool market
    /// [R] Rent sysvar
    /// [R] Everlend ULP program id
    InitPoolMarket,

    /// Creates and initializes a pool account belonging to a particular market
    ///
    /// Accounts:
    /// [R] Income pool market
    /// [W] Income pool
    /// [R] Token mint
    /// [W] Token account
    /// [WS] Market manager
    /// [R] Income pool market authority
    /// [R] Rent sysvar
    /// [R] Sytem program
    /// [R] Token program id
    CreatePool,

    /// Deposit funds in the pool
    ///
    /// Accounts:
    /// [R] Income pool market
    /// [R] Income pool
    /// [W] Source account (for token mint)
    /// [W] Token account
    /// [RS] User transfer authority
    /// [R] Token program id
    Deposit {
        /// Amount to deposit
        amount: u64,
    },

    /// Withdraw funds from the pool
    ///
    /// Accounts:
    /// [R] Income pool market
    /// [R] Income pool
    /// [W] Token account
    /// [R] Income pool market authority
    /// [R] General pool
    /// [W] General pool token account
    /// [R] Everlend ULP program id
    /// [R] Token program id
    Withdraw,
}

/// Creates 'InitPoolMarket' instruction.
#[allow(clippy::too_many_arguments)]
pub fn init_pool_market(
    program_id: &Pubkey,
    pool_market: &Pubkey,
    manager: &Pubkey,
    general_pool_market: &Pubkey,
) -> Instruction {
    let accounts = vec![
        AccountMeta::new(*pool_market, false),
        AccountMeta::new_readonly(*manager, false),
        AccountMeta::new_readonly(*general_pool_market, false),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
        AccountMeta::new_readonly(everlend_ulp::id(), false),
    ];

    Instruction::new_with_borsh(
        *program_id,
        &IncomePoolsInstruction::InitPoolMarket,
        accounts,
    )
}

/// Creates 'CreatePool' instruction.
#[allow(clippy::too_many_arguments)]
pub fn create_pool(
    program_id: &Pubkey,
    pool_market: &Pubkey,
    token_mint: &Pubkey,
    token_account: &Pubkey,
    manager: &Pubkey,
) -> Instruction {
    let (pool_market_authority, _) = find_program_address(program_id, pool_market);
    let (pool, _) = find_pool_program_address(program_id, pool_market, token_mint);

    let accounts = vec![
        AccountMeta::new_readonly(*pool_market, false),
        AccountMeta::new(pool, false),
        AccountMeta::new_readonly(*token_mint, false),
        AccountMeta::new(*token_account, false),
        AccountMeta::new(*manager, true),
        AccountMeta::new_readonly(pool_market_authority, false),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(spl_token::id(), false),
    ];

    Instruction::new_with_borsh(*program_id, &IncomePoolsInstruction::CreatePool, accounts)
}

/// Creates 'Deposit' instruction.
#[allow(clippy::too_many_arguments)]
pub fn deposit(
    program_id: &Pubkey,
    pool_market: &Pubkey,
    pool: &Pubkey,
    source: &Pubkey,
    token_account: &Pubkey,
    user_transfer_authority: &Pubkey,
    amount: u64,
) -> Instruction {
    let accounts = vec![
        AccountMeta::new_readonly(*pool_market, false),
        AccountMeta::new_readonly(*pool, false),
        AccountMeta::new(*source, false),
        AccountMeta::new(*token_account, false),
        AccountMeta::new_readonly(*user_transfer_authority, true),
        AccountMeta::new_readonly(spl_token::id(), false),
    ];

    Instruction::new_with_borsh(
        *program_id,
        &IncomePoolsInstruction::Deposit { amount },
        accounts,
    )
}

/// Creates 'Withdraw' instruction.
#[allow(clippy::too_many_arguments)]
pub fn withdraw(
    program_id: &Pubkey,
    pool_market: &Pubkey,
    pool: &Pubkey,
    token_account: &Pubkey,
    general_pool: &Pubkey,
    general_pool_token_account: &Pubkey,
) -> Instruction {
    let (pool_market_authority, _) = find_program_address(program_id, pool_market);

    let accounts = vec![
        AccountMeta::new_readonly(*pool_market, false),
        AccountMeta::new_readonly(*pool, false),
        AccountMeta::new(*token_account, false),
        AccountMeta::new_readonly(pool_market_authority, false),
        AccountMeta::new_readonly(*general_pool, false),
        AccountMeta::new_readonly(*general_pool_token_account, false),
        AccountMeta::new_readonly(everlend_ulp::id(), false),
        AccountMeta::new_readonly(spl_token::id(), false),
    ];

    Instruction::new_with_borsh(*program_id, &IncomePoolsInstruction::Withdraw, accounts)
}
