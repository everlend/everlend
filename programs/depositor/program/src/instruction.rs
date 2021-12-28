//! Instruction types

use crate::{find_rebalancing_program_address, find_transit_program_address};
use borsh::{BorshDeserialize, BorshSerialize};
use everlend_liquidity_oracle::find_liquidity_oracle_token_distribution_program_address;
use everlend_ulp::{find_pool_borrow_authority_program_address, find_pool_program_address};
use everlend_utils::find_program_address;
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_program, sysvar,
};

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

    /// Start rebalancing
    ///
    /// Accounts:
    /// [R] Depositor
    /// [W] Rebalancing account
    /// [R] Token mint
    /// [R] Pool market
    /// [R] Pool
    /// [R] Pool token account
    /// [R] Liquidity oracle
    /// [R] Token distribution
    /// [WS] From account
    /// [R] Rent sysvar
    /// [R] Sytem program
    /// [R] Everlend Liquidity Oracle program id
    /// [R] Everlend ULP program id
    StartRebalancing,

    /// Deposit funds from General Pool to Money market.
    /// Collect collateral token to MM Pool.
    ///
    /// Accounts:
    /// [R] Depositor
    /// [R] Depositor authority
    /// [W] Rebalancing account
    /// [R] Pool market
    /// [R] Pool market authority
    /// [R] Pool
    /// [R] Pool borrow authority
    /// [W] Pool token account (for liquidity mint)
    /// [R] MM Pool market
    /// [R] MM Pool market authority
    /// [R] MM Pool
    /// [W] MM Pool token account (for collateral mint)
    /// [W] MM Pool collateral transit account
    /// [W] MM Pool collateral mint
    /// [W] Liquidity transit account
    /// [R] Liquidity mint
    /// [W] Collateral transit account
    /// [W] Collateral mint
    /// [R] Sysvar clock program id
    /// [R] Everlend ULP program id
    /// [R] Token program id
    /// [R] Money market program id
    Deposit,

    /// Withdraw funds from MM Pool to Money market.
    /// Collect liquidity token to General Pool.
    ///
    /// Accounts:
    /// [R] Depositor
    /// [R] Depositor authority
    /// [W] Rebalancing account
    /// [R] Pool market
    /// [R] Pool market authority
    /// [R] Pool
    /// [R] Pool borrow authority
    /// [W] Pool token account (for liquidity mint)
    /// [R] MM Pool market
    /// [R] MM Pool market authority
    /// [R] MM Pool
    /// [W] MM Pool token account (for collateral mint)
    /// [W] MM Pool collateral transit account
    /// [W] MM Pool collateral mint
    /// [W] Liquidity transit account
    /// [R] Liquidity mint
    /// [W] Collateral transit account
    /// [W] Collateral mint
    /// [R] Sysvar clock program id
    /// [R] Everlend ULP program id
    /// [R] Token program id
    /// [R] Money market program id
    Withdraw,
}

/// Creates 'Init' instruction.
#[allow(clippy::too_many_arguments)]
pub fn init(
    program_id: &Pubkey,
    depositor: &Pubkey,
    pool_market: &Pubkey,
    liquidity_oracle: &Pubkey,
) -> Instruction {
    let accounts = vec![
        AccountMeta::new(*depositor, false),
        AccountMeta::new_readonly(*pool_market, false),
        AccountMeta::new_readonly(*liquidity_oracle, false),
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

/// Creates 'StartRebalancing' instruction.
#[allow(clippy::too_many_arguments)]
pub fn start_rebalancing(
    program_id: &Pubkey,
    depositor: &Pubkey,
    mint: &Pubkey,
    pool_market: &Pubkey,
    pool_token_account: &Pubkey,
    liquidity_oracle: &Pubkey,
    from: &Pubkey,
) -> Instruction {
    let (rebalancing, _) = find_rebalancing_program_address(program_id, depositor, mint);
    let (token_distribution, _) = find_liquidity_oracle_token_distribution_program_address(
        &everlend_liquidity_oracle::id(),
        liquidity_oracle,
        mint,
    );
    let (pool, _) = find_pool_program_address(&everlend_ulp::id(), pool_market, mint);

    let accounts = vec![
        AccountMeta::new_readonly(*depositor, false),
        AccountMeta::new(rebalancing, false),
        AccountMeta::new_readonly(*mint, false),
        AccountMeta::new_readonly(*pool_market, false),
        AccountMeta::new_readonly(pool, false),
        AccountMeta::new_readonly(*pool_token_account, false),
        AccountMeta::new_readonly(*liquidity_oracle, false),
        AccountMeta::new_readonly(token_distribution, false),
        AccountMeta::new(*from, true),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(everlend_liquidity_oracle::id(), false),
        AccountMeta::new_readonly(everlend_ulp::id(), false),
    ];

    Instruction::new_with_borsh(
        *program_id,
        &DepositorInstruction::StartRebalancing,
        accounts,
    )
}

/// Creates 'Deposit' instruction.
#[allow(clippy::too_many_arguments)]
pub fn deposit(
    program_id: &Pubkey,
    depositor: &Pubkey,
    pool_market: &Pubkey,
    pool_token_account: &Pubkey,
    mm_pool_market: &Pubkey,
    mm_pool_token_account: &Pubkey,
    mm_pool_collateral_mint: &Pubkey,
    liquidity_mint: &Pubkey,
    collateral_mint: &Pubkey,
    money_market_program_id: &Pubkey,
    money_market_accounts: Vec<AccountMeta>,
) -> Instruction {
    let (depositor_authority, _) = find_program_address(program_id, depositor);
    let (rebalancing, _) = find_rebalancing_program_address(program_id, depositor, liquidity_mint);

    let (pool_market_authority, _) = find_program_address(&everlend_ulp::id(), pool_market);
    let (pool, _) = find_pool_program_address(&everlend_ulp::id(), pool_market, liquidity_mint);
    let (pool_borrow_authority, _) = find_pool_borrow_authority_program_address(
        &everlend_ulp::id(),
        &pool,
        &depositor_authority,
    );

    let (mm_pool_market_authority, _) = find_program_address(&everlend_ulp::id(), mm_pool_market);
    let (mm_pool, _) =
        find_pool_program_address(&everlend_ulp::id(), mm_pool_market, collateral_mint);

    let (liquidity_transit, _) =
        find_transit_program_address(program_id, depositor, liquidity_mint);
    let (collateral_transit, _) =
        find_transit_program_address(program_id, depositor, collateral_mint);
    let (mm_pool_collateral_transit, _) =
        find_transit_program_address(program_id, depositor, mm_pool_collateral_mint);

    let mut accounts = vec![
        AccountMeta::new_readonly(*depositor, false),
        AccountMeta::new_readonly(depositor_authority, false),
        AccountMeta::new(rebalancing, false),
        // Pool
        AccountMeta::new_readonly(*pool_market, false),
        AccountMeta::new_readonly(pool_market_authority, false),
        AccountMeta::new(pool, false),
        AccountMeta::new(pool_borrow_authority, false),
        AccountMeta::new(*pool_token_account, false),
        // Money market pool
        AccountMeta::new_readonly(*mm_pool_market, false),
        AccountMeta::new_readonly(mm_pool_market_authority, false),
        AccountMeta::new_readonly(mm_pool, false),
        AccountMeta::new(*mm_pool_token_account, false),
        AccountMeta::new(mm_pool_collateral_transit, false),
        AccountMeta::new(*mm_pool_collateral_mint, false),
        // Common
        AccountMeta::new(liquidity_transit, false),
        AccountMeta::new_readonly(*liquidity_mint, false),
        AccountMeta::new(collateral_transit, false),
        AccountMeta::new(*collateral_mint, false),
        // Programs
        AccountMeta::new_readonly(sysvar::clock::id(), false),
        AccountMeta::new_readonly(everlend_ulp::id(), false),
        AccountMeta::new_readonly(spl_token::id(), false),
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
    depositor: &Pubkey,
    pool_market: &Pubkey,
    pool_token_account: &Pubkey,
    mm_pool_market: &Pubkey,
    mm_pool_token_account: &Pubkey,
    mm_pool_collateral_mint: &Pubkey,
    collateral_mint: &Pubkey,
    liquidity_mint: &Pubkey,
    money_market_program_id: &Pubkey,
    money_market_accounts: Vec<AccountMeta>,
) -> Instruction {
    let (depositor_authority, _) = find_program_address(program_id, depositor);
    let (rebalancing, _) = find_rebalancing_program_address(program_id, depositor, liquidity_mint);

    let (pool_market_authority, _) = find_program_address(&everlend_ulp::id(), pool_market);
    let (pool, _) = find_pool_program_address(&everlend_ulp::id(), pool_market, liquidity_mint);
    let (pool_borrow_authority, _) = find_pool_borrow_authority_program_address(
        &everlend_ulp::id(),
        &pool,
        &depositor_authority,
    );

    let (mm_pool_market_authority, _) = find_program_address(&everlend_ulp::id(), mm_pool_market);
    let (mm_pool, _) =
        find_pool_program_address(&everlend_ulp::id(), mm_pool_market, collateral_mint);

    let (collateral_transit, _) =
        find_transit_program_address(program_id, depositor, collateral_mint);
    let (liquidity_transit, _) =
        find_transit_program_address(program_id, depositor, liquidity_mint);
    let (mm_pool_collateral_transit, _) =
        find_transit_program_address(program_id, depositor, mm_pool_collateral_mint);

    let mut accounts = vec![
        AccountMeta::new_readonly(*depositor, false),
        AccountMeta::new_readonly(depositor_authority, false),
        AccountMeta::new(rebalancing, false),
        // Pool
        AccountMeta::new_readonly(*pool_market, false),
        AccountMeta::new_readonly(pool_market_authority, false),
        AccountMeta::new(pool, false),
        AccountMeta::new(pool_borrow_authority, false),
        AccountMeta::new(*pool_token_account, false),
        // Money market pool
        AccountMeta::new_readonly(*mm_pool_market, false),
        AccountMeta::new_readonly(mm_pool_market_authority, false),
        AccountMeta::new_readonly(mm_pool, false),
        AccountMeta::new(*mm_pool_token_account, false),
        AccountMeta::new(mm_pool_collateral_transit, false),
        AccountMeta::new(*mm_pool_collateral_mint, false),
        // Common
        AccountMeta::new(collateral_transit, false),
        AccountMeta::new(*collateral_mint, false),
        AccountMeta::new(liquidity_transit, false),
        AccountMeta::new_readonly(*liquidity_mint, false),
        // Programs
        AccountMeta::new_readonly(sysvar::clock::id(), false),
        AccountMeta::new_readonly(everlend_ulp::id(), false),
        AccountMeta::new_readonly(spl_token::id(), false),
        // Money market
        AccountMeta::new_readonly(*money_market_program_id, false),
    ];

    accounts.extend(money_market_accounts);

    Instruction::new_with_borsh(*program_id, &DepositorInstruction::Withdraw, accounts)
}
