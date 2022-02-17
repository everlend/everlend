//! Instruction types

use crate::{
    find_pool_borrow_authority_program_address, find_pool_program_address,
    find_transit_program_address, find_user_withdrawal_request_program_address, find_withdrawal_requests_program_address,
};
use borsh::{BorshDeserialize, BorshSerialize};
use everlend_utils::find_program_address;
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_program, sysvar,
};

/// Instructions supported by the program
#[derive(Debug, BorshDeserialize, BorshSerialize, PartialEq)]
pub enum LiquidityPoolsInstruction {
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
    /// [W] Withdrawals requests account
    /// [R] Token mint
    /// [W] Token account
    /// [W] Transit collateral account
    /// [W] Pool mint
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
    /// [W] Pool borrow authority
    /// [RS] Market manager
    UpdatePoolBorrowAuthority {
        /// Share allowed
        share_allowed: u16,
    },

    /// Delete a pool borrow authority
    ///
    /// Accounts:
    /// [R] Pool market
    /// [W] Pool borrow authority
    /// [W] Receiver lamports
    /// [RS] Market manager
    DeletePoolBorrowAuthority,

    /// Deposit funds in the pool
    ///
    /// Accounts:
    /// [R] Pool market
    /// [R] Pool
    /// [W] Source account (for token mint)
    /// [W] Destination account (for pool mint)
    /// [W] Token account
    /// [W] Pool mint account
    /// [R] Pool market authority
    /// [RS] User transfer authority
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
    /// [W] Withdrawals requests account
    /// [W] Destination account
    /// [W] Pool token account
    /// [W] Transit collateral account
    /// [W] Pool mint account
    /// [R] Pool market authority
    /// [W] Rent payer
    /// [R] Token program id
    Withdraw,

    /// Borrow funds from the pool
    ///
    /// Accounts:
    /// [R] Pool market
    /// [W] Pool
    /// [W] Pool borrow authority
    /// [W] Destination account (for token mint)
    /// [W] Token account
    /// [W] Withdrawal requests account
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

    /// Move pool tokens to transit account and create withdraw request
    ///
    /// Accounts:
    /// [R] Pool market
    /// [R] Pool
    /// [W] Withdrawals requests account
    /// [W] User withdraw request account
    /// [W] Source account (for pool mint)
    /// [R] Destination account (for token mint)
    /// [W] Token account
    /// [W] Transit collateral account
    /// [W] Pool mint account
    /// [RS] User transfer authority
    /// [R] Rent sysvar
    /// [R] System program
    /// [R] Token program id
    WithdrawRequest {
        /// Amount to withdraw
        amount: u64,
    },

    /// Cancel withdraw request and return collateral tokens to user
    ///
    /// Accounts:
    /// [R] Pool market
    /// [R] Pool
    /// [W] Withdrawals requests account
    /// [W] User withdraw request account
    /// [W] Withdrawal source collateral account
    /// [W] Transit collateral account
    /// [W] Pool mint account
    /// [R] Pool market authority
    /// [W] Rent payer
    /// [RS] Market manager
    /// [R] Token program id
    CancelWithdrawRequest
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
        &LiquidityPoolsInstruction::InitPoolMarket,
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
    pool_mint: &Pubkey,
    manager: &Pubkey,
) -> Instruction {
    let (pool_market_authority, _) = find_program_address(program_id, pool_market);
    let (pool, _) = find_pool_program_address(program_id, pool_market, token_mint);
    let (transit_collateral, _) = find_transit_program_address(program_id, pool_market, pool_mint);
    let (withdrawal_requests, _) =
        find_withdrawal_requests_program_address(program_id, pool_market, token_mint);

    let accounts = vec![
        AccountMeta::new_readonly(*pool_market, false),
        AccountMeta::new(pool, false),
        AccountMeta::new(withdrawal_requests, false),
        AccountMeta::new_readonly(*token_mint, false),
        AccountMeta::new(*token_account, false),
        AccountMeta::new(transit_collateral, false),
        AccountMeta::new(*pool_mint, false),
        AccountMeta::new(*manager, true),
        AccountMeta::new_readonly(pool_market_authority, false),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(spl_token::id(), false),
    ];

    Instruction::new_with_borsh(
        *program_id,
        &LiquidityPoolsInstruction::CreatePool,
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
        &LiquidityPoolsInstruction::CreatePoolBorrowAuthority { share_allowed },
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
        AccountMeta::new(pool_borrow_authority, false),
        AccountMeta::new_readonly(*manager, true),
    ];

    Instruction::new_with_borsh(
        *program_id,
        &LiquidityPoolsInstruction::UpdatePoolBorrowAuthority { share_allowed },
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
        AccountMeta::new(pool_borrow_authority, false),
        AccountMeta::new(*receiver, false),
        AccountMeta::new_readonly(*manager, true),
    ];

    Instruction::new_with_borsh(
        *program_id,
        &LiquidityPoolsInstruction::DeletePoolBorrowAuthority,
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
    destination: &Pubkey,
    token_account: &Pubkey,
    pool_mint: &Pubkey,
    user_transfer_authority: &Pubkey,
    amount: u64,
) -> Instruction {
    let (pool_market_authority, _) = find_program_address(program_id, pool_market);

    let accounts = vec![
        AccountMeta::new_readonly(*pool_market, false),
        AccountMeta::new_readonly(*pool, false),
        AccountMeta::new(*source, false),
        AccountMeta::new(*destination, false),
        AccountMeta::new(*token_account, false),
        AccountMeta::new(*pool_mint, false),
        AccountMeta::new_readonly(pool_market_authority, false),
        AccountMeta::new_readonly(*user_transfer_authority, true),
        AccountMeta::new_readonly(spl_token::id(), false),
    ];

    Instruction::new_with_borsh(
        *program_id,
        &LiquidityPoolsInstruction::Deposit { amount },
        accounts,
    )
}

/// Creates 'Withdraw' instruction.
#[allow(clippy::too_many_arguments)]
pub fn withdraw(
    program_id: &Pubkey,
    pool_market: &Pubkey,
    pool: &Pubkey,
    destination: &Pubkey,
    token_account: &Pubkey,
    token_mint: &Pubkey,
    pool_mint: &Pubkey,
    rent_payer: &Pubkey,
    index: u64,
) -> Instruction {
    let (pool_market_authority, _) = find_program_address(program_id, pool_market);

    let (withdrawal_requests, _) =
        find_withdrawal_requests_program_address(program_id, pool_market, token_mint);
    let (collateral_transit, _) = find_transit_program_address(program_id, pool_market, pool_mint);
    let (user_withdrawal_request, _) =  find_user_withdrawal_request_program_address(program_id, pool_market, token_mint, index);

    let accounts = vec![
        AccountMeta::new_readonly(*pool_market, false),
        AccountMeta::new_readonly(*pool, false),
        AccountMeta::new(withdrawal_requests, false),
        AccountMeta::new(user_withdrawal_request, false),
        AccountMeta::new(*destination, false),
        AccountMeta::new(*token_account, false),
        AccountMeta::new(collateral_transit, false),
        AccountMeta::new(*pool_mint, false),
        AccountMeta::new_readonly(pool_market_authority, false),
        AccountMeta::new(*rent_payer, false),
        AccountMeta::new_readonly(spl_token::id(), false),
    ];

    Instruction::new_with_borsh(
        *program_id,
        &LiquidityPoolsInstruction::Withdraw,
        accounts,
    )
}

/// Creates 'WithdrawRequest' instruction.
#[allow(clippy::too_many_arguments)]
pub fn withdraw_request(
    program_id: &Pubkey,
    pool_market: &Pubkey,
    pool: &Pubkey,
    source: &Pubkey,
    destination: &Pubkey,
    token_account: &Pubkey,
    token_mint: &Pubkey,
    pool_mint: &Pubkey,
    user_transfer_authority: &Pubkey,
    amount: u64,
    index: u64,
) -> Instruction {
    let (withdrawal_requests, _) =
        find_withdrawal_requests_program_address(program_id, pool_market, token_mint);
    let (collateral_transit, _) = find_transit_program_address(program_id, pool_market, pool_mint);
    let (user_withdrawal_request, _) =  find_user_withdrawal_request_program_address(program_id, pool_market, token_mint, index);

    let accounts = vec![
        AccountMeta::new_readonly(*pool_market, false),
        AccountMeta::new_readonly(*pool, false),
        AccountMeta::new(withdrawal_requests, false),
        AccountMeta::new(user_withdrawal_request, false),
        AccountMeta::new(*source, false),
        AccountMeta::new_readonly(*destination, false),
        AccountMeta::new(*token_account, false),
        AccountMeta::new(collateral_transit, false),
        AccountMeta::new(*pool_mint, false),
        AccountMeta::new(*user_transfer_authority, true),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(spl_token::id(), false),
    ];

    Instruction::new_with_borsh(
        *program_id,
        &LiquidityPoolsInstruction::WithdrawRequest { amount },
        accounts,
    )
}

/// Creates 'CancelWithdrawRequest' instruction.
#[allow(clippy::too_many_arguments)]
pub fn cancel_withdraw_request(
    program_id: &Pubkey,
    pool_market: &Pubkey,
    pool: &Pubkey,
    source: &Pubkey,
    token_mint: &Pubkey,
    pool_mint: &Pubkey,
    manager_authority: &Pubkey,
    rent_payer: &Pubkey,
    index: u64,
) -> Instruction {
    let (pool_market_authority, _) = find_program_address(program_id, pool_market);

    let (withdrawal_requests, _) =
        find_withdrawal_requests_program_address(program_id, pool_market, token_mint);
    let (collateral_transit, _) = find_transit_program_address(program_id, pool_market, pool_mint);
    let (user_withdrawal_request, _) =  find_user_withdrawal_request_program_address(program_id, pool_market, token_mint, index);


    let accounts = vec![
        AccountMeta::new_readonly(*pool_market, false),
        AccountMeta::new_readonly(*pool, false),
        AccountMeta::new(withdrawal_requests, false),
        AccountMeta::new(user_withdrawal_request, false),
        AccountMeta::new(*source, false),
        AccountMeta::new(collateral_transit, false),
        AccountMeta::new(*pool_mint, false),
        AccountMeta::new_readonly(pool_market_authority, false),
        AccountMeta::new(*rent_payer, false),
        AccountMeta::new_readonly(*manager_authority, true),
        AccountMeta::new_readonly(spl_token::id(), false),
    ];

    Instruction::new_with_borsh(
        *program_id,
        &LiquidityPoolsInstruction::CancelWithdrawRequest,
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
        &LiquidityPoolsInstruction::Borrow { amount },
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
        &LiquidityPoolsInstruction::Repay {
            amount,
            interest_amount,
        },
        accounts,
    )
}
