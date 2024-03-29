//! Instruction types

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_program, sysvar,
};

use everlend_utils::find_program_address;

use crate::{
    find_pool_borrow_authority_program_address, find_pool_config_program_address,
    find_pool_program_address, find_transit_program_address,
    find_withdrawal_request_program_address, find_withdrawal_requests_program_address,
    state::SetPoolConfigParams,
};

/// Instructions supported by the program
#[derive(Debug, BorshDeserialize, BorshSerialize, PartialEq)]
pub enum LiquidityPoolsInstruction {
    /// Initializes a new pool market
    ///
    /// Accounts:
    /// [W] Pool market - uninitialized
    /// [WS] Market manager
    /// [R] Registry
    /// [R] Rent sysvar
    InitPoolMarket,

    /// Creates and initializes a pool account belonging to a particular market
    ///
    /// Accounts:
    /// [R] Pool market
    /// [W] Pool
    /// [W] Pool Config
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
    /// [R] Pool market
    /// [R] Pool
    /// [W] Pool borrow authority
    /// [W] Receiver lamports
    /// [RS] Market manager
    DeletePoolBorrowAuthority,

    /// Deposit funds in the pool
    ///
    /// Accounts:
    /// [R] Pool config
    /// [R] Pool market
    /// [R] Pool
    /// [W] Source token account (for token mint)
    /// [W] Destination token account (for pool mint)
    /// [W] Pool token account
    /// [W] Pool mint account
    /// [R] Pool market authority
    /// [RS] User transfer authority
    /// [W] Mining reward pool
    /// [W] User mining reward account
    /// [R] Everlend rewards program
    /// [R] Token program id
    Deposit {
        /// Amount to deposit
        amount: u64,
    },

    /// Burn pool tokens and withdraw funds from the pool
    ///
    /// Accounts:
    /// [R] Pool market
    /// [R] Pool market authority
    /// [R] Pool
    /// [W] Pool mint account
    /// [W] Withdrawal requests account
    /// [W] Withdrawal request account
    /// [W] Destination account
    /// [W] Pool token account
    /// [W] Collateral transit account
    /// [W] From account
    /// [R] Clock sysvar
    /// [R] Token program id
    /// Additional accounts for SOL unwrap
    /// [R] Token mint
    /// [W] Unwrap SOL token account
    /// [S] Signer
    /// [R] Rent sysvar
    /// [R] System program
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
    /// [R] Pool config
    /// [R] Pool market
    /// [R] Pool
    /// [W] Pool mint account
    /// [W] Withdrawal requests account
    /// [W] Withdrawal request account
    /// [W] Source token account (for pool mint)
    /// [R] Destination token  account (for token mint)
    /// [W] Pool token account
    /// [W] Collateral transit account
    /// [RS] User transfer authority
    /// [W] Mining reward pool
    /// [W] User mining reward account
    /// [R] Everlend rewards program
    /// [R] Rent sysvar
    /// [R] Clock sysvar
    /// [R] System program
    /// [R] Token program id
    WithdrawRequest {
        /// Collateral amount
        collateral_amount: u64,
    },

    /// Cancel withdraw request and return collateral tokens to user
    ///
    /// Accounts:
    /// [R] Pool market
    /// [R] Pool
    /// [W] Withdrawal requests account
    /// [W] Withdrawal request account
    /// [W] Withdrawal source collateral account
    /// [W] Transit collateral account
    /// [W] Pool mint account
    /// [R] Pool market authority
    /// [W] From account
    /// [RS] Market manager
    /// [R] Token program id
    CancelWithdrawRequest,

    /// Migrate PoolMarket
    ///
    /// Accounts:
    /// [W] Pool market
    /// [R] Manager
    ClosePoolMarket,

    /// Migrate account data
    ///
    MigrationInstruction,

    /// Init user mining account
    ///
    InitUserMining,

    /// Update pool market manager
    ///
    /// Accounts:
    /// [W] Pool market
    /// [WS] Old manager
    /// [RS] New manager
    ///
    UpdateManager,

    /// SetTokenMetadata
    ///
    SetTokenMetadata {
        /// The on-chain name of the token, limited to 32 bytes. For instance "Degen Ape #1".
        name: String,
        /// The on-chain symbol of the token, limited to 10 bytes. For instance "DAPE".
        symbol: String,
        /// The URI of the token, limited to 200 bytes. This URI points to an off-chain JSON file that contains additional data
        uri: String,
    },

    /// Set pool config
    ///
    /// Accounts:
    /// [R] Pool Market
    /// [R] Pool
    /// [W] Pool config
    /// [WS] Manager
    /// [R] Rent sysvar
    /// [R] System program
    SetPoolConfig {
        /// Pool config update params
        params: SetPoolConfigParams,
    },

    /// Move pool tokens to destination user account
    ///
    /// Accounts:
    /// [R] Pool
    /// [W] Source account
    /// [W] Destination account
    /// [RS] User transfer authority
    /// [R] Destination user transfer authority
    /// [W] Mining reward pool
    /// [W] Mining reward user account
    /// [W] Destination mining reward user account
    /// [R] Everlend rewards program account
    /// [R] Token program id
    TransferDeposit,
}

/// Creates 'InitPoolMarket' instruction.
#[allow(clippy::too_many_arguments)]
pub fn init_pool_market(
    program_id: &Pubkey,
    pool_market: &Pubkey,
    manager: &Pubkey,
    registry: &Pubkey,
) -> Instruction {
    let accounts = vec![
        AccountMeta::new(*pool_market, false),
        AccountMeta::new(*manager, true),
        AccountMeta::new_readonly(*registry, false),
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
    let (pool_config, _) = find_pool_config_program_address(program_id, &pool);
    let (transit_collateral, _) = find_transit_program_address(program_id, pool_market, pool_mint);
    let (withdrawal_requests, _) =
        find_withdrawal_requests_program_address(program_id, pool_market, token_mint);

    let accounts = vec![
        AccountMeta::new_readonly(*pool_market, false),
        AccountMeta::new(pool, false),
        AccountMeta::new(pool_config, false),
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
        AccountMeta::new_readonly(*pool, false),
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
        AccountMeta::new_readonly(*pool, false),
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
    mining_reward_pool: &Pubkey,
    mining_reward_acc: &Pubkey,
    amount: u64,
) -> Instruction {
    let (pool_market_authority, _) = find_program_address(program_id, pool_market);
    let (pool_config, _) = find_pool_config_program_address(program_id, pool);

    let accounts = vec![
        AccountMeta::new_readonly(pool_config, false),
        AccountMeta::new_readonly(*pool_market, false),
        AccountMeta::new_readonly(*pool, false),
        AccountMeta::new(*source, false),
        AccountMeta::new(*destination, false),
        AccountMeta::new(*token_account, false),
        AccountMeta::new(*pool_mint, false),
        AccountMeta::new_readonly(pool_market_authority, false),
        AccountMeta::new_readonly(*user_transfer_authority, true),
        AccountMeta::new(*mining_reward_pool, false),
        AccountMeta::new(*mining_reward_acc, false),
        AccountMeta::new_readonly(everlend_rewards::id(), false),
        AccountMeta::new_readonly(spl_token::id(), false),
    ];

    Instruction::new_with_borsh(
        *program_id,
        &LiquidityPoolsInstruction::Deposit { amount },
        accounts,
    )
}

/// Creates 'TransferDeposit' instruction
#[allow(clippy::too_many_arguments)]
pub fn transfer_deposit(
    program_id: &Pubkey,
    pool: &Pubkey,
    source: &Pubkey,
    destination: &Pubkey,
    user_transfer_authority: &Pubkey,
    destination_user_transfer_authority: &Pubkey,
    mining_reward_pool: &Pubkey,
    mining_reward_acc: &Pubkey,
    destination_mining_reward_acc: &Pubkey,
) -> Instruction {
    let accounts = vec![
        AccountMeta::new_readonly(*pool, false),
        AccountMeta::new(*source, false),
        AccountMeta::new(*destination, false),
        AccountMeta::new_readonly(*user_transfer_authority, true),
        AccountMeta::new_readonly(*destination_user_transfer_authority, false),
        AccountMeta::new(*mining_reward_pool, false),
        AccountMeta::new(*mining_reward_acc, false),
        AccountMeta::new(*destination_mining_reward_acc, false),
        AccountMeta::new_readonly(everlend_rewards::id(), false),
        AccountMeta::new_readonly(spl_token::id(), false),
    ];

    Instruction::new_with_borsh(
        *program_id,
        &LiquidityPoolsInstruction::TransferDeposit,
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
    from: &Pubkey,
    addition_accounts: Vec<AccountMeta>,
) -> Instruction {
    let (pool_market_authority, _) = find_program_address(program_id, pool_market);

    let (withdrawal_requests, _) =
        find_withdrawal_requests_program_address(program_id, pool_market, token_mint);
    let (withdrawal_request, _) =
        find_withdrawal_request_program_address(program_id, &withdrawal_requests, from);
    let (collateral_transit, _) = find_transit_program_address(program_id, pool_market, pool_mint);

    let mut accounts = vec![
        AccountMeta::new_readonly(*pool_market, false),
        AccountMeta::new_readonly(pool_market_authority, false),
        AccountMeta::new_readonly(*pool, false),
        AccountMeta::new(*pool_mint, false),
        AccountMeta::new(withdrawal_requests, false),
        AccountMeta::new(withdrawal_request, false),
        AccountMeta::new(*destination, false),
        AccountMeta::new(*token_account, false),
        AccountMeta::new(collateral_transit, false),
        AccountMeta::new(*from, false),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
        AccountMeta::new_readonly(spl_token::id(), false),
    ];

    accounts.extend(addition_accounts);

    Instruction::new_with_borsh(*program_id, &LiquidityPoolsInstruction::Withdraw, accounts)
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
    mining_reward_pool: &Pubkey,
    mining_reward_acc: &Pubkey,
    collateral_amount: u64,
) -> Instruction {
    let (withdrawal_requests, _) =
        find_withdrawal_requests_program_address(program_id, pool_market, token_mint);
    let (collateral_transit, _) = find_transit_program_address(program_id, pool_market, pool_mint);
    let (withdrawal_request, _) = find_withdrawal_request_program_address(
        program_id,
        &withdrawal_requests,
        user_transfer_authority,
    );
    let (pool_config, _) = find_pool_config_program_address(program_id, pool);

    let accounts = vec![
        AccountMeta::new_readonly(pool_config, false),
        AccountMeta::new_readonly(*pool_market, false),
        AccountMeta::new_readonly(*pool, false),
        AccountMeta::new(*pool_mint, false),
        AccountMeta::new(withdrawal_requests, false),
        AccountMeta::new(withdrawal_request, false),
        AccountMeta::new(*source, false),
        AccountMeta::new_readonly(*destination, false),
        AccountMeta::new(*token_account, false),
        AccountMeta::new(collateral_transit, false),
        AccountMeta::new(*user_transfer_authority, true),
        AccountMeta::new(*mining_reward_pool, false),
        AccountMeta::new(*mining_reward_acc, false),
        AccountMeta::new_readonly(everlend_rewards::id(), false),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(spl_token::id(), false),
    ];

    Instruction::new_with_borsh(
        *program_id,
        &LiquidityPoolsInstruction::WithdrawRequest { collateral_amount },
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
    manager: &Pubkey,
    from: &Pubkey,
) -> Instruction {
    let (pool_market_authority, _) = find_program_address(program_id, pool_market);

    let (withdrawal_requests, _) =
        find_withdrawal_requests_program_address(program_id, pool_market, token_mint);
    let (collateral_transit, _) = find_transit_program_address(program_id, pool_market, pool_mint);
    let (withdrawal_request, _) =
        find_withdrawal_request_program_address(program_id, &withdrawal_requests, from);

    let accounts = vec![
        AccountMeta::new_readonly(*pool_market, false),
        AccountMeta::new_readonly(*pool, false),
        AccountMeta::new(withdrawal_requests, false),
        AccountMeta::new(withdrawal_request, false),
        AccountMeta::new(*source, false),
        AccountMeta::new(collateral_transit, false),
        AccountMeta::new(*pool_mint, false),
        AccountMeta::new_readonly(pool_market_authority, false),
        AccountMeta::new(*from, false),
        AccountMeta::new_readonly(*manager, true),
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

/// Creates 'MigratePoolMarket' instruction.
#[allow(clippy::too_many_arguments)]
pub fn close_pool_market(
    program_id: &Pubkey,
    pool_market: &Pubkey,
    manager: &Pubkey,
) -> Instruction {
    let accounts = vec![
        AccountMeta::new(*pool_market, false),
        AccountMeta::new_readonly(*manager, false),
    ];

    Instruction::new_with_borsh(
        *program_id,
        &LiquidityPoolsInstruction::ClosePoolMarket,
        accounts,
    )
}

/// Creates 'Borrow' instruction.
#[allow(clippy::too_many_arguments)]
pub fn init_user_mining(
    program_id: &Pubkey,
    pool_market: &Pubkey,
    pool: &Pubkey,
    user_collateral_token_account: &Pubkey,
    user_authority: &Pubkey,
    manager: &Pubkey,
    mining_reward_pool: &Pubkey,
    mining_reward_acc: &Pubkey,
    config: &Pubkey,
) -> Instruction {
    let accounts = vec![
        AccountMeta::new_readonly(*pool_market, false),
        AccountMeta::new_readonly(*pool, false),
        AccountMeta::new_readonly(*user_collateral_token_account, false),
        AccountMeta::new_readonly(*user_authority, false),
        AccountMeta::new(*manager, true),
        AccountMeta::new(*mining_reward_pool, false),
        AccountMeta::new(*mining_reward_acc, false),
        AccountMeta::new_readonly(*config, false),
        AccountMeta::new_readonly(everlend_rewards::id(), false),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
    ];

    Instruction::new_with_borsh(
        *program_id,
        &LiquidityPoolsInstruction::InitUserMining,
        accounts,
    )
}

/// Creates 'Migration' instruction.
#[allow(clippy::too_many_arguments)]
pub fn migrate_instruction(program_id: &Pubkey) -> Instruction {
    let accounts = vec![];

    Instruction::new_with_borsh(
        *program_id,
        &LiquidityPoolsInstruction::MigrationInstruction,
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
        &LiquidityPoolsInstruction::UpdateManager,
        accounts,
    )
}

/// Creates 'SetRegistryPoolConfig' instruction.
pub fn set_pool_config(
    program_id: &Pubkey,
    pool_market: &Pubkey,
    pool: &Pubkey,
    manager: &Pubkey,
    params: SetPoolConfigParams,
) -> Instruction {
    let (pool_config, _) = find_pool_config_program_address(program_id, pool);

    let accounts = vec![
        AccountMeta::new_readonly(*pool_market, false),
        AccountMeta::new_readonly(*pool, false),
        AccountMeta::new(pool_config, false),
        AccountMeta::new(*manager, true),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
        AccountMeta::new_readonly(system_program::id(), false),
    ];

    Instruction::new_with_borsh(
        *program_id,
        &LiquidityPoolsInstruction::SetPoolConfig { params },
        accounts,
    )
}
