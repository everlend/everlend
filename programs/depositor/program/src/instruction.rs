//! Instruction types

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    sysvar,
};

/// Instructions supported by the program
#[derive(Debug, BorshDeserialize, BorshSerialize, PartialEq)]
pub enum DepositorInstruction {
    /// Deposit funds from ULP to money market
    ///
    /// Accounts:
    /// [R] ULP Pool market
    /// [R] ULP Pool
    /// [R] ULP Pool borrow authority
    /// [W] ULP token account (for token mint)
    /// [W] Staging token account (for token mint)
    /// [RS] Depositor
    /// [R] Sysvar instructions program id
    /// [R] Token program id
    Deposit {
        /// Amount to deposit
        amount: u64,
    },
}

/// Creates 'Deposit' instruction.
#[allow(clippy::too_many_arguments)]
pub fn deposit(
    program_id: &Pubkey,
    ulp_pool_market: &Pubkey,
    ulp_pool: &Pubkey,
    ulp_pool_borrow_authority: &Pubkey,
    ulp_token_account: &Pubkey,
    staging_token_account: &Pubkey,
    depositor: &Pubkey,
    amount: u64,
) -> Instruction {
    let accounts = vec![
        AccountMeta::new_readonly(*ulp_pool_market, false),
        AccountMeta::new_readonly(*ulp_pool, false),
        AccountMeta::new_readonly(*ulp_pool_borrow_authority, false),
        AccountMeta::new(*ulp_token_account, false),
        AccountMeta::new(*staging_token_account, false),
        AccountMeta::new_readonly(*depositor, true),
        AccountMeta::new_readonly(sysvar::instructions::id(), false),
        AccountMeta::new_readonly(spl_token::id(), false),
    ];

    Instruction::new_with_borsh(
        *program_id,
        &DepositorInstruction::Deposit { amount },
        accounts,
    )
}
