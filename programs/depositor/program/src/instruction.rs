//! Instruction types

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_program, sysvar,
};

use crate::{find_program_address, find_transit_program_address};

/// Instructions supported by the program
#[derive(Debug, BorshDeserialize, BorshSerialize, PartialEq)]
pub enum DepositorInstruction {
    /// Initializes a new depositor
    ///
    /// Accounts:
    /// [W] Depositor account - uninitialized
    /// [R] Rent sysvar
    Init,

    /// Create transit token account for liquidity
    ///
    /// Accounts:
    /// [R] Depositor account
    /// [W] Transit account
    /// [R] Token mint
    /// [R] Depositor authority
    /// [WS] From account
    /// [R] Rent sysvar
    /// [R] Sytem program
    /// [R] Token program id
    CreateTransit,

    /// Deposit funds from ULP to money market
    ///
    /// Accounts:
    /// [R] Depositor
    /// [R] Pool market
    /// [R] Pool
    /// [R] Pool borrow authority
    /// [W] Pool token account (for token mint)
    /// [W] Transit token account (for token mint)
    /// [R] Token mint
    /// [RS] Rebalancer
    /// [R] Sysvar instructions program id
    /// [R] Token program id
    Deposit {
        /// Amount to deposit
        amount: u64,
    },
}

/// Creates 'Init' instruction.
#[allow(clippy::too_many_arguments)]
pub fn init(program_id: &Pubkey, depositor: &Pubkey) -> Instruction {
    let accounts = vec![
        AccountMeta::new(*depositor, false),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
    ];

    Instruction::new_with_borsh(*program_id, &DepositorInstruction::Init, accounts)
}

/// Creates 'CreateTransit' instruction.
#[allow(clippy::too_many_arguments)]
pub fn create_transit(
    program_id: &Pubkey,
    depositor: &Pubkey,
    mint: &Pubkey,
    from: &Pubkey,
) -> Instruction {
    let (depositor_authority, _) = find_program_address(program_id, depositor);
    let (transit, _) = find_transit_program_address(program_id, depositor, mint);

    let accounts = vec![
        AccountMeta::new_readonly(*depositor, false),
        AccountMeta::new(transit, false),
        AccountMeta::new_readonly(*mint, false),
        AccountMeta::new_readonly(depositor_authority, false),
        AccountMeta::new(*from, true),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(spl_token::id(), false),
    ];

    Instruction::new_with_borsh(*program_id, &DepositorInstruction::CreateTransit, accounts)
}

/// Creates 'Deposit' instruction.
#[allow(clippy::too_many_arguments)]
pub fn deposit(
    program_id: &Pubkey,
    depositor: &Pubkey,
    pool_market: &Pubkey,
    pool: &Pubkey,
    pool_borrow_authority: &Pubkey,
    pool_token_account: &Pubkey,
    liquidity_mint: &Pubkey,
    rebalancer: &Pubkey,
    amount: u64,
) -> Instruction {
    let (pool_market_authority, _) = find_program_address(&everlend_ulp::id(), pool_market);
    let (liquidity_transit, _) =
        find_transit_program_address(program_id, depositor, liquidity_mint);

    let accounts = vec![
        AccountMeta::new_readonly(*depositor, false),
        AccountMeta::new_readonly(*pool_market, false),
        AccountMeta::new(*pool, false),
        AccountMeta::new(*pool_borrow_authority, false),
        AccountMeta::new_readonly(pool_market_authority, false),
        AccountMeta::new(*pool_token_account, false),
        AccountMeta::new(liquidity_transit, false),
        AccountMeta::new_readonly(*liquidity_mint, false),
        AccountMeta::new_readonly(*rebalancer, true),
        AccountMeta::new_readonly(everlend_ulp::id(), false),
        AccountMeta::new_readonly(sysvar::instructions::id(), false),
        AccountMeta::new_readonly(spl_token::id(), false),
    ];

    Instruction::new_with_borsh(
        *program_id,
        &DepositorInstruction::Deposit { amount },
        accounts,
    )
}
