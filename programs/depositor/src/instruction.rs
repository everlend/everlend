//! Instruction types

use borsh::{BorshDeserialize, BorshSerialize};
use everlend_collateral_pool::find_pool_withdraw_authority_program_address;
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_program, sysvar,
};

use everlend_general_pool::find_withdrawal_requests_program_address;
use everlend_liquidity_oracle::find_liquidity_oracle_token_distribution_program_address;
use everlend_liquidity_oracle::state::DistributionArray;
use everlend_utils::find_program_address;

use crate::{find_rebalancing_program_address, find_transit_program_address};

/// Instructions supported by the program
#[derive(Debug, BorshDeserialize, BorshSerialize, PartialEq)]
pub enum DepositorInstruction {
    /// Initializes a new depositor
    ///
    /// Accounts:
    /// [W] Depositor account - uninitialized
    /// [R] Registry
    /// [R] Rent sysvar
    Init {
        /// Rebalance executor account
        rebalance_executor: Pubkey,
    },

    /// Create transit token account for liquidity
    ///
    /// Accounts:
    /// [R] Depositor account
    /// [W] Transit account
    /// [R] Token mint
    /// [R] Depositor authority
    /// [WS] From account
    /// [R] Rent sysvar
    /// [R] System program
    /// [R] Token program id
    CreateTransit {
        /// Seed
        seed: String,
    },

    /// Computing rebalancing steps and updating the liquidity on the transit account
    ///
    /// Accounts:
    /// [R] Registry config
    /// [R] Depositor
    /// [R] Depositor authority
    /// [W] Rebalancing account
    /// [R] Token mint
    /// [R] General pool market
    /// [R] General pool market authority
    /// [W] General pool
    /// [W] General pool token account
    /// [W] General pool borrow authority
    /// [R] Withdrawals requests account
    /// [W] Liquidity transit account
    /// [R] Liquidity oracle
    /// [R] Token distribution
    /// [WS] Rebalance executor account
    /// [R] Rent sysvar
    /// [R] Clock sysvar
    /// [R] System program
    /// [R] Token program id
    /// [R] Everlend Liquidity Oracle program id
    /// [R] Everlend general pool program id
    StartRebalancing {
        /// Refresh income
        refresh_income: bool,
    },

    /// Deposit funds from liquidity transit account to money market.
    /// Collect collateral token to MM pool.
    ///
    /// Accounts:
    /// [R] Registry config
    /// [R] Depositor
    /// [R] Depositor authority
    /// [W] Rebalancing account
    /// [R] MM Pool market
    /// [R] MM Pool market authority
    /// [R] MM Pool
    /// [W] MM Pool token account (for collateral mint)
    /// [W] Liquidity transit account
    /// [R] Liquidity mint
    /// [W] Collateral transit account
    /// [W] Collateral mint
    /// [S] Rebalance executor account
    /// [R] Clock sysvar
    /// [R] Token program id
    /// [R] Everlend collateral pool program id
    /// [R] Money market program id
    Deposit,

    /// Withdraw funds from MM pool to money market.
    /// Collect liquidity token to liquidity transit account.
    ///
    /// Accounts:
    /// [R] Registry config
    /// [R] Depositor
    /// [R] Depositor authority
    /// [W] Rebalancing account
    /// [R] Income pool market
    /// [R] Income pool
    /// [W] Income pool token account (for liquidity mint)
    /// [R] MM Pool market
    /// [R] MM Pool market authority
    /// [R] MM Pool
    /// [W] MM Pool token account (for collateral mint)
    /// [R] MM Pool withdraw authority
    /// [W] Collateral transit account
    /// [W] Collateral mint
    /// [W] Liquidity transit account
    /// [W] Liquidity reserve transit account
    /// [R] Liquidity mint
    /// [S] Rebalance executor account
    /// [R] Clock sysvar
    /// [R] Token program id
    /// [R] Everlend collateral program id
    /// [R] Money market program id
    Withdraw,

    /// Migrate depositor to v1
    ///
    /// Accounts
    /// [W] Depositor
    /// [R] Registry
    /// [R] Manager
    /// [R] Rent
    MigrateDepositor {
        /// Rebalancing executor account
        rebalance_executor: Pubkey,
    },

    /// Set current rebalancing
    ///
    /// Accounts:
    /// [R] Registry
    /// [R] Depositor
    /// [W] Rebalancing account
    /// [R] Token mint
    /// [WS] Manager
    /// [R] System program
    SetRebalancing {
        ///Manual setup of prev distributed liquidity
        distributed_liquidity: u64,
        ///Manual setup of prev distribution array
        distribution_array: DistributionArray,
    },
}

/// Creates 'Init' instruction.
#[allow(clippy::too_many_arguments)]
pub fn init(
    program_id: &Pubkey,
    registry: &Pubkey,
    depositor: &Pubkey,
    rebalance_executor: &Pubkey,
) -> Instruction {
    let accounts = vec![
        AccountMeta::new(*depositor, false),
        AccountMeta::new_readonly(*registry, false),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
    ];

    Instruction::new_with_borsh(
        *program_id,
        &DepositorInstruction::Init {
            rebalance_executor: *rebalance_executor,
        },
        accounts,
    )
}

/// Creates 'CreateTransit' instruction.
#[allow(clippy::too_many_arguments)]
pub fn create_transit(
    program_id: &Pubkey,
    depositor: &Pubkey,
    mint: &Pubkey,
    from: &Pubkey,
    //todo! remove option
    seed: Option<String>,
) -> Instruction {
    let seed = seed.unwrap_or_default();
    let (depositor_authority, _) = find_program_address(program_id, depositor);
    let (transit, _) = find_transit_program_address(program_id, depositor, mint, &seed);

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

    Instruction::new_with_borsh(
        *program_id,
        &DepositorInstruction::CreateTransit { seed },
        accounts,
    )
}

/// Creates 'StartRebalancing' instruction.
#[allow(clippy::too_many_arguments)]
pub fn start_rebalancing(
    program_id: &Pubkey,
    registry: &Pubkey,
    depositor: &Pubkey,
    mint: &Pubkey,
    general_pool_market: &Pubkey,
    general_pool_token_account: &Pubkey,
    liquidity_oracle: &Pubkey,
    rebalance_executor: &Pubkey,
    refresh_income: bool,
) -> Instruction {
    let (registry_config, _) =
        everlend_registry::find_config_program_address(&everlend_registry::id(), registry);

    let (depositor_authority, _) = find_program_address(program_id, depositor);
    let (rebalancing, _) = find_rebalancing_program_address(program_id, depositor, mint);
    let (token_distribution, _) = find_liquidity_oracle_token_distribution_program_address(
        &everlend_liquidity_oracle::id(),
        liquidity_oracle,
        mint,
    );
    // General pool
    let (general_pool_market_authority, _) =
        find_program_address(&everlend_general_pool::id(), general_pool_market);
    let (general_pool, _) = everlend_general_pool::find_pool_program_address(
        &everlend_general_pool::id(),
        general_pool_market,
        mint,
    );
    let (general_pool_borrow_authority, _) =
        everlend_general_pool::find_pool_borrow_authority_program_address(
            &everlend_general_pool::id(),
            &general_pool,
            &depositor_authority,
        );
    let (withdrawal_requests, _) = find_withdrawal_requests_program_address(
        &everlend_general_pool::id(),
        general_pool_market,
        mint,
    );

    let (liquidity_transit, _) = find_transit_program_address(program_id, depositor, mint, "");

    let accounts = vec![
        AccountMeta::new_readonly(registry_config, false),
        AccountMeta::new_readonly(*depositor, false),
        AccountMeta::new_readonly(depositor_authority, false),
        AccountMeta::new(rebalancing, false),
        AccountMeta::new_readonly(*mint, false),
        AccountMeta::new_readonly(*general_pool_market, false),
        AccountMeta::new_readonly(general_pool_market_authority, false),
        AccountMeta::new(general_pool, false),
        AccountMeta::new(*general_pool_token_account, false),
        AccountMeta::new(general_pool_borrow_authority, false),
        AccountMeta::new_readonly(withdrawal_requests, false),
        AccountMeta::new(liquidity_transit, false),
        AccountMeta::new_readonly(*liquidity_oracle, false),
        AccountMeta::new_readonly(token_distribution, false),
        AccountMeta::new(*rebalance_executor, true),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(spl_token::id(), false),
        AccountMeta::new_readonly(everlend_liquidity_oracle::id(), false),
        AccountMeta::new_readonly(everlend_general_pool::id(), false),
    ];

    Instruction::new_with_borsh(
        *program_id,
        &DepositorInstruction::StartRebalancing { refresh_income },
        accounts,
    )
}

/// Creates 'ResetRebalancing' instruction.
#[allow(clippy::too_many_arguments)]
pub fn reset_rebalancing(
    program_id: &Pubkey,
    registry: &Pubkey,
    depositor: &Pubkey,
    liquidity_mint: &Pubkey,
    manager: &Pubkey,
    distributed_liquidity: u64,
    distribution_array: DistributionArray,
) -> Instruction {
    let (rebalancing, _) = find_rebalancing_program_address(program_id, depositor, liquidity_mint);

    let accounts = vec![
        AccountMeta::new_readonly(*registry, false),
        AccountMeta::new_readonly(*depositor, false),
        AccountMeta::new(rebalancing, false),
        AccountMeta::new_readonly(*liquidity_mint, false),
        AccountMeta::new_readonly(*manager, true),
        AccountMeta::new_readonly(system_program::id(), false),
    ];

    Instruction::new_with_borsh(
        *program_id,
        &DepositorInstruction::SetRebalancing {
            distributed_liquidity,
            distribution_array,
        },
        accounts,
    )
}

/// Creates 'Deposit' instruction.
#[allow(clippy::too_many_arguments)]
pub fn deposit(
    program_id: &Pubkey,
    registry: &Pubkey,
    depositor: &Pubkey,
    mm_pool_market: &Pubkey,
    mm_pool_token_account: &Pubkey,
    liquidity_mint: &Pubkey,
    collateral_mint: &Pubkey,
    rebalance_executor: &Pubkey,
    money_market_program_id: &Pubkey,
    money_market_accounts: Vec<AccountMeta>,
) -> Instruction {
    let (registry_config, _) =
        everlend_registry::find_config_program_address(&everlend_registry::id(), registry);
    let (depositor_authority, _) = find_program_address(program_id, depositor);
    let (rebalancing, _) = find_rebalancing_program_address(program_id, depositor, liquidity_mint);

    // MM pool
    let (mm_pool_market_authority, _) =
        find_program_address(&everlend_collateral_pool::id(), mm_pool_market);
    let (mm_pool, _) = everlend_collateral_pool::find_pool_program_address(
        &everlend_collateral_pool::id(),
        mm_pool_market,
        collateral_mint,
    );

    let (liquidity_transit, _) =
        find_transit_program_address(program_id, depositor, liquidity_mint, "");
    let (collateral_transit, _) =
        find_transit_program_address(program_id, depositor, collateral_mint, "");

    let mut accounts = vec![
        AccountMeta::new_readonly(registry_config, false),
        AccountMeta::new_readonly(*depositor, false),
        AccountMeta::new_readonly(depositor_authority, false),
        AccountMeta::new(rebalancing, false),
        // Money market pool
        AccountMeta::new_readonly(*mm_pool_market, false),
        AccountMeta::new_readonly(mm_pool_market_authority, false),
        AccountMeta::new_readonly(mm_pool, false),
        AccountMeta::new(*mm_pool_token_account, false),
        // Common
        AccountMeta::new(liquidity_transit, false),
        AccountMeta::new_readonly(*liquidity_mint, false),
        AccountMeta::new(collateral_transit, false),
        AccountMeta::new(*collateral_mint, false),
        AccountMeta::new_readonly(*rebalance_executor, true),
        // Programs
        AccountMeta::new_readonly(sysvar::clock::id(), false),
        AccountMeta::new_readonly(spl_token::id(), false),
        AccountMeta::new_readonly(everlend_collateral_pool::id(), false),
        // Money market
        AccountMeta::new_readonly(*money_market_program_id, false),
    ];

    accounts.extend(money_market_accounts);

    Instruction::new_with_borsh(*program_id, &DepositorInstruction::Deposit, accounts)
}

/// Creates 'Withdraw' instruction.
#[allow(clippy::too_many_arguments)]
pub fn withdraw(
    program_id: &Pubkey,
    registry: &Pubkey,
    depositor: &Pubkey,
    income_pool_market: &Pubkey,
    income_pool_token_account: &Pubkey,
    mm_pool_market: &Pubkey,
    mm_pool_token_account: &Pubkey,
    collateral_mint: &Pubkey,
    liquidity_mint: &Pubkey,
    rebalance_executor: &Pubkey,
    money_market_program_id: &Pubkey,
    money_market_accounts: Vec<AccountMeta>,
) -> Instruction {
    let (registry_config, _) =
        everlend_registry::find_config_program_address(&everlend_registry::id(), registry);
    let (depositor_authority, _) = find_program_address(program_id, depositor);
    let (rebalancing, _) = find_rebalancing_program_address(program_id, depositor, liquidity_mint);

    // Income pool
    let (income_pool, _) = everlend_income_pools::find_pool_program_address(
        &everlend_income_pools::id(),
        income_pool_market,
        liquidity_mint,
    );

    // MM pool
    let (mm_pool_market_authority, _) =
        find_program_address(&everlend_collateral_pool::id(), mm_pool_market);
    let (mm_pool, _) = everlend_collateral_pool::find_pool_program_address(
        &everlend_collateral_pool::id(),
        mm_pool_market,
        collateral_mint,
    );

    let (collateral_transit, _) =
        find_transit_program_address(program_id, depositor, collateral_mint, "");
    let (liquidity_transit, _) =
        find_transit_program_address(program_id, depositor, liquidity_mint, "");

    let (liquidity_reserve_transit, _) =
        find_transit_program_address(program_id, depositor, liquidity_mint, "reserve");

    let (mm_pool_withdraw_authority, _) = find_pool_withdraw_authority_program_address(
        &everlend_collateral_pool::id(),
        &mm_pool,
        &depositor_authority,
    );

    let mut accounts = vec![
        AccountMeta::new_readonly(registry_config, false),
        AccountMeta::new_readonly(*depositor, false),
        AccountMeta::new_readonly(depositor_authority, false),
        AccountMeta::new(rebalancing, false),
        // Income pool
        AccountMeta::new_readonly(*income_pool_market, false),
        AccountMeta::new_readonly(income_pool, false),
        AccountMeta::new(*income_pool_token_account, false),
        // Money market pool
        AccountMeta::new_readonly(*mm_pool_market, false),
        AccountMeta::new_readonly(mm_pool_market_authority, false),
        AccountMeta::new_readonly(mm_pool, false),
        AccountMeta::new(*mm_pool_token_account, false),
        AccountMeta::new(mm_pool_withdraw_authority, false),
        // Common
        AccountMeta::new(collateral_transit, false),
        AccountMeta::new(*collateral_mint, false),
        AccountMeta::new(liquidity_transit, false),
        AccountMeta::new(liquidity_reserve_transit, false),
        AccountMeta::new_readonly(*liquidity_mint, false),
        AccountMeta::new_readonly(*rebalance_executor, true),
        // Programs
        AccountMeta::new_readonly(sysvar::clock::id(), false),
        AccountMeta::new_readonly(spl_token::id(), false),
        AccountMeta::new_readonly(everlend_income_pools::id(), false),
        AccountMeta::new_readonly(everlend_collateral_pool::id(), false),
        // Money market
        AccountMeta::new_readonly(*money_market_program_id, false),
    ];

    accounts.extend(money_market_accounts);

    Instruction::new_with_borsh(*program_id, &DepositorInstruction::Withdraw, accounts)
}

/// Creates 'MigrateDepositor' instruction.
#[allow(clippy::too_many_arguments)]
pub fn migrate_depositor(
    program_id: &Pubkey,
    depositor: &Pubkey,
    registry: &Pubkey,
    manager: &Pubkey,
    rebalance_executor: &Pubkey,
) -> Instruction {
    let accounts = vec![
        AccountMeta::new(*depositor, false),
        AccountMeta::new_readonly(*registry, false),
        AccountMeta::new_readonly(*manager, false),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
    ];

    Instruction::new_with_borsh(
        *program_id,
        &DepositorInstruction::MigrateDepositor {
            rebalance_executor: *rebalance_executor,
        },
        accounts,
    )
}
