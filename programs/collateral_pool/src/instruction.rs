//! Instruction types

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_program, sysvar,
};

use everlend_utils::find_program_address;

use crate::{
    find_pool_borrow_authority_program_address, find_pool_program_address,
    find_pool_withdraw_authority_program_address,
};

/// Instructions supported by the program
#[derive(Debug, BorshDeserialize, BorshSerialize, PartialEq)]
pub enum CollateralPoolsInstruction {
    /// Initializes a new pool market
    ///
    /// Accounts:
    /// [W] Pool market - uninitialized
    /// [R] Market manager
    /// [R] Rent sysvar
    InitPoolMarket,

    /// Creates and initializes a pool account belonging to a particular market
    ///
    /// Accounts:
    /// [R] Pool market
    /// [W] Pool
    /// [R] Token mint
    /// [W] Token account
    /// [WS] Market manager
    /// [R] Pool market authority
    /// [R] Rent sysvar
    /// [R] System program
    /// [R] Token program id
    CreatePool,

    /// Creates and initializes a pool borrow authority
    ///
    /// Accounts:
    /// [R] Pool market
    /// [R] Pool
    /// [W] Pool borrow authority
    /// [R] Borrow authority
    /// [WS] Market manager
    /// [R] Rent sysvar
    /// [R] System program
    CreatePoolBorrowAuthority {
        /// Share allowed
        share_allowed: u16,
    },

    /// Update a pool borrow authority
    ///
    /// Accounts:
    /// [R] Pool market
    /// [R] Pool
    /// [W] Pool borrow authority
    /// [RS] Market manager
    UpdatePoolBorrowAuthority {
        /// Share allowed
        share_allowed: u16,
    },

    /// Delete a pool borrow authority
    ///
    /// Accounts:
    /// [W] Pool borrow authority
    /// [R] Pool
    /// [W] Receiver lamports
    /// [RS] Market manager
    DeletePoolBorrowAuthority,

    /// Creates and initializes a pool withdraw authority
    ///
    /// Accounts:
    /// [R] Pool market
    /// [R] Pool
    /// [W] Pool withdraw authority
    /// [WS] Market manager
    /// [R] Rent sysvar
    /// [R] System program
    CreatePoolWithdrawAuthority,

    /// Delete a pool withdraw authority
    ///
    /// Accounts:
    /// [W] Pool withdraw authority
    /// [R] Pool
    /// [W] Receiver lamports
    /// [RS] Market manager
    DeletePoolWithdrawAuthority,

    /// Deposit funds in the pool
    ///
    /// Accounts:
    /// [R] Pool market
    /// [R] Pool
    /// [W] Source account (for token mint)
    /// [W] Token account
    /// [R] Pool market authority
    /// [R] Token program id
    Deposit {
        /// Amount to deposit
        amount: u64,
    },

    /// Burn pool tokens and withdraw funds from the pool
    ///
    /// Accounts:
    /// [R] Pool market
    /// [R] Pool
    /// [R] Pool withdraw authority
    /// [W] Destination account (for token mint)
    /// [W] Token account
    /// [R] Pool market authority
    /// [RS] Withdraw authority
    /// [R] Token program id
    Withdraw {
        /// Amount to withdraw
        amount: u64,
    },

    /// Borrow funds from the pool
    ///
    /// Accounts:
    /// [R] Pool market
    /// [W] Pool
    /// [W] Pool borrow authority
    /// [W] Destination account (for token mint)
    /// [W] Token account
    /// [R] Pool market authority
    /// [RS] Borrow authority
    /// [R] Token program id
    Borrow {
        /// Amount to borrow
        amount: u64,
    },

    /// Repay funds back to the pool
    ///
    /// Accounts:
    /// [R] Pool market
    /// [W] Pool
    /// [W] Pool borrow authority
    /// [W] Source account (for token mint)
    /// [W] Token account
    /// [RS] User transfer authority
    /// [R] Token program id
    Repay {
        /// Amount to repay
        amount: u64,
        /// Interest amount
        interest_amount: u64,
    },

    /// Update pool market manager
    ///
    /// Accounts:
    /// [W] Pool market
    /// [WS] Old manager
    /// [RS] New manager
    ///
    UpdateManager,
}

/// Creates 'InitPoolMarket' instruction.
#[allow(clippy::too_many_arguments)]
pub fn init_pool_market(
    program_id: &Pubkey,
    pool_market: &Pubkey,
    manager: &Pubkey,
) -> Instruction {
    let accounts = vec![
        AccountMeta::new(*pool_market, false),
        AccountMeta::new_readonly(*manager, false),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
    ];

    Instruction::new_with_borsh(
        *program_id,
        &CollateralPoolsInstruction::InitPoolMarket,
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

    Instruction::new_with_borsh(
        *program_id,
        &CollateralPoolsInstruction::CreatePool,
        accounts,
    )
}

/// Creates 'CreatePoolBorrowAuthority' instruction.
#[allow(clippy::too_many_arguments)]
pub fn create_pool_borrow_authority(
    program_id: &Pubkey,
    pool_market: &Pubkey,
    pool: &Pubkey,
    borrow_authority: &Pubkey,
    manager: &Pubkey,
    share_allowed: u16,
) -> Instruction {
    let (pool_borrow_authority, _) =
        find_pool_borrow_authority_program_address(program_id, pool, borrow_authority);

    let accounts = vec![
        AccountMeta::new_readonly(*pool_market, false),
        AccountMeta::new_readonly(*pool, false),
        AccountMeta::new(pool_borrow_authority, false),
        AccountMeta::new_readonly(*borrow_authority, false),
        AccountMeta::new(*manager, true),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
        AccountMeta::new_readonly(system_program::id(), false),
    ];

    Instruction::new_with_borsh(
        *program_id,
        &CollateralPoolsInstruction::CreatePoolBorrowAuthority { share_allowed },
        accounts,
    )
}

/// Creates 'UpdatePoolBorrowAuthority' instruction.
#[allow(clippy::too_many_arguments)]
pub fn update_pool_borrow_authority(
    program_id: &Pubkey,
    pool_market: &Pubkey,
    pool: &Pubkey,
    borrow_authority: &Pubkey,
    manager: &Pubkey,
    share_allowed: u16,
) -> Instruction {
    let (pool_borrow_authority, _) =
        find_pool_borrow_authority_program_address(program_id, pool, borrow_authority);

    let accounts = vec![
        AccountMeta::new_readonly(*pool_market, false),
        AccountMeta::new_readonly(*pool, false),
        AccountMeta::new(pool_borrow_authority, false),
        AccountMeta::new_readonly(*manager, true),
    ];

    Instruction::new_with_borsh(
        *program_id,
        &CollateralPoolsInstruction::UpdatePoolBorrowAuthority { share_allowed },
        accounts,
    )
}

/// Creates 'DeletePoolBorrowAuthority' instruction.
#[allow(clippy::too_many_arguments)]
pub fn delete_pool_borrow_authority(
    program_id: &Pubkey,
    pool_market: &Pubkey,
    pool: &Pubkey,
    borrow_authority: &Pubkey,
    receiver: &Pubkey,
    manager: &Pubkey,
) -> Instruction {
    let (pool_borrow_authority, _) =
        find_pool_borrow_authority_program_address(program_id, pool, borrow_authority);

    let accounts = vec![
        AccountMeta::new_readonly(*pool_market, false),
        AccountMeta::new_readonly(*pool, false),
        AccountMeta::new(pool_borrow_authority, false),
        AccountMeta::new(*receiver, false),
        AccountMeta::new_readonly(*manager, true),
    ];

    Instruction::new_with_borsh(
        *program_id,
        &CollateralPoolsInstruction::DeletePoolBorrowAuthority,
        accounts,
    )
}

/// Creates 'CreatePoolWithdrawAuthority' instruction.
#[allow(clippy::too_many_arguments)]
pub fn create_pool_withdraw_authority(
    program_id: &Pubkey,
    pool_market: &Pubkey,
    pool: &Pubkey,
    withdraw_authority: &Pubkey,
    manager: &Pubkey,
) -> Instruction {
    let (pool_withdraw_authority, _) =
        find_pool_withdraw_authority_program_address(program_id, pool, withdraw_authority);

    let accounts = vec![
        AccountMeta::new_readonly(*pool_market, false),
        AccountMeta::new_readonly(*pool, false),
        AccountMeta::new(pool_withdraw_authority, false),
        AccountMeta::new_readonly(*withdraw_authority, false),
        AccountMeta::new(*manager, true),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
        AccountMeta::new_readonly(system_program::id(), false),
    ];

    Instruction::new_with_borsh(
        *program_id,
        &CollateralPoolsInstruction::CreatePoolWithdrawAuthority,
        accounts,
    )
}

/// Creates 'DeletePoolWithdrawAuthority' instruction.
#[allow(clippy::too_many_arguments)]
pub fn delete_pool_withdraw_authority(
    program_id: &Pubkey,
    pool_market: &Pubkey,
    pool: &Pubkey,
    withdraw_authority: &Pubkey,
    receiver: &Pubkey,
    manager: &Pubkey,
) -> Instruction {
    let (pool_withdraw_authority, _) =
        find_pool_withdraw_authority_program_address(program_id, pool, withdraw_authority);

    let accounts = vec![
        AccountMeta::new_readonly(*pool_market, false),
        AccountMeta::new_readonly(*pool, false),
        AccountMeta::new(pool_withdraw_authority, false),
        AccountMeta::new(*receiver, false),
        AccountMeta::new_readonly(*manager, true),
    ];

    Instruction::new_with_borsh(
        *program_id,
        &CollateralPoolsInstruction::DeletePoolWithdrawAuthority,
        accounts,
    )
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
        &CollateralPoolsInstruction::Deposit { amount },
        accounts,
    )
}

/// Creates 'Withdraw' instruction.
#[allow(clippy::too_many_arguments)]
pub fn withdraw(
    program_id: &Pubkey,
    pool_market: &Pubkey,
    pool: &Pubkey,
    pool_withdraw_authority: &Pubkey,
    destination: &Pubkey,
    token_account: &Pubkey,
    user_transfer_authority: &Pubkey,
    amount: u64,
) -> Instruction {
    let (pool_market_authority, _) = find_program_address(program_id, pool_market);

    let accounts = vec![
        AccountMeta::new_readonly(*pool_market, false),
        AccountMeta::new_readonly(*pool, false),
        AccountMeta::new_readonly(*pool_withdraw_authority, false),
        AccountMeta::new(*destination, false),
        AccountMeta::new(*token_account, false),
        AccountMeta::new_readonly(pool_market_authority, false),
        AccountMeta::new_readonly(*user_transfer_authority, true),
        AccountMeta::new_readonly(spl_token::id(), false),
    ];

    Instruction::new_with_borsh(
        *program_id,
        &CollateralPoolsInstruction::Withdraw { amount },
        accounts,
    )
}

/// Creates 'Borrow' instruction.
#[allow(clippy::too_many_arguments)]
pub fn borrow(
    program_id: &Pubkey,
    pool_market: &Pubkey,
    pool: &Pubkey,
    pool_borrow_authority: &Pubkey,
    destination: &Pubkey,
    token_account: &Pubkey,
    borrow_authority: &Pubkey,
    amount: u64,
) -> Instruction {
    let (pool_market_authority, _) = find_program_address(program_id, pool_market);

    let accounts = vec![
        AccountMeta::new_readonly(*pool_market, false),
        AccountMeta::new(*pool, false),
        AccountMeta::new(*pool_borrow_authority, false),
        AccountMeta::new(*destination, false),
        AccountMeta::new(*token_account, false),
        AccountMeta::new_readonly(pool_market_authority, false),
        AccountMeta::new_readonly(*borrow_authority, true),
        AccountMeta::new_readonly(spl_token::id(), false),
    ];

    Instruction::new_with_borsh(
        *program_id,
        &CollateralPoolsInstruction::Borrow { amount },
        accounts,
    )
}

/// Creates 'Repay' instruction.
#[allow(clippy::too_many_arguments)]
pub fn repay(
    program_id: &Pubkey,
    pool_market: &Pubkey,
    pool: &Pubkey,
    pool_borrow_authority: &Pubkey,
    source: &Pubkey,
    token_account: &Pubkey,
    user_transfer_authority: &Pubkey,
    amount: u64,
    interest_amount: u64,
) -> Instruction {
    let accounts = vec![
        AccountMeta::new_readonly(*pool_market, false),
        AccountMeta::new(*pool, false),
        AccountMeta::new(*pool_borrow_authority, false),
        AccountMeta::new(*source, false),
        AccountMeta::new(*token_account, false),
        AccountMeta::new_readonly(*user_transfer_authority, true),
        AccountMeta::new_readonly(spl_token::id(), false),
    ];

    Instruction::new_with_borsh(
        *program_id,
        &CollateralPoolsInstruction::Repay {
            amount,
            interest_amount,
        },
        accounts,
    )
}

/// Creates 'UpdateManager' instruction.
#[allow(clippy::too_many_arguments)]
pub fn update_manager(
    program_id: &Pubkey,
    pool_market: &Pubkey,
    manager: &Pubkey,
    new_manager: &Pubkey,
) -> Instruction {
    let accounts = vec![
        AccountMeta::new(*pool_market, false),
        AccountMeta::new(*manager, true),
        AccountMeta::new_readonly(*new_manager, true),
    ];

    Instruction::new_with_borsh(
        *program_id,
        &CollateralPoolsInstruction::UpdateManager,
        accounts,
    )
}
