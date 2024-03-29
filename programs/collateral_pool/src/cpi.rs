//! CPI

use solana_program::{
    account_info::AccountInfo, program::invoke_signed, program_error::ProgramError,
};
use super::utils::CollateralPoolAccounts;

/// Borrow collateral tokens
#[allow(clippy::too_many_arguments)]
pub fn borrow<'a>(
    pool_market: AccountInfo<'a>,
    pool_market_authority: AccountInfo<'a>,
    pool: AccountInfo<'a>,
    pool_borrow_authority: AccountInfo<'a>,
    destination: AccountInfo<'a>,
    token_account: AccountInfo<'a>,
    borrow_authority: AccountInfo<'a>,
    amount: u64,
    signers_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    let ix = crate::instruction::borrow(
        &crate::id(),
        pool_market.key,
        pool.key,
        pool_borrow_authority.key,
        destination.key,
        token_account.key,
        borrow_authority.key,
        amount,
    );

    invoke_signed(
        &ix,
        &[
            pool_market,
            pool,
            pool_borrow_authority,
            pool_market_authority,
            destination,
            token_account,
            borrow_authority,
        ],
        signers_seeds,
    )
}

/// Repay collateral tokens
#[allow(clippy::too_many_arguments)]
pub fn repay<'a>(
    pool_market: AccountInfo<'a>,
    pool_market_authority: AccountInfo<'a>,
    pool: AccountInfo<'a>,
    pool_borrow_authority: AccountInfo<'a>,
    source: AccountInfo<'a>,
    token_account: AccountInfo<'a>,
    authority: AccountInfo<'a>,
    amount: u64,
    interest_amount: u64,
    signers_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    let ix = crate::instruction::repay(
        &crate::id(),
        pool_market.key,
        pool.key,
        pool_borrow_authority.key,
        source.key,
        token_account.key,
        authority.key,
        amount,
        interest_amount,
    );

    invoke_signed(
        &ix,
        &[
            pool_market,
            pool,
            pool_borrow_authority,
            pool_market_authority,
            source,
            token_account,
            authority,
        ],
        signers_seeds,
    )
}

/// Deposit collateral tokens
#[allow(clippy::too_many_arguments)]
pub fn deposit<'a>(
    collateral_pool_accounts: CollateralPoolAccounts<'a>,
    source: AccountInfo<'a>,
    user_transfer_authority: AccountInfo<'a>,
    amount: u64,
    signers_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    let ix = crate::instruction::deposit(
        &crate::id(),
        collateral_pool_accounts.pool_market.key,
        collateral_pool_accounts.pool.key,
        source.key,
        collateral_pool_accounts.token_account.key,
        user_transfer_authority.key,
        amount,
    );

    invoke_signed(
        &ix,
        &[
            collateral_pool_accounts.pool_market,
            collateral_pool_accounts.pool,
            source,
            collateral_pool_accounts.token_account,
            collateral_pool_accounts.pool_market_authority,
            user_transfer_authority,
        ],
        signers_seeds,
    )
}

/// Withdraw collateral tokens
#[allow(clippy::too_many_arguments)]
pub fn withdraw<'a>(
    collateral_pool_accounts: CollateralPoolAccounts<'a>,
    collateral_pool_withdraw_authority: AccountInfo<'a>,
    destination: AccountInfo<'a>,
    user_transfer_authority: AccountInfo<'a>,
    amount: u64,
    signers_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    let ix = crate::instruction::withdraw(
        &crate::id(),
        collateral_pool_accounts.pool_market.key,
        collateral_pool_accounts.pool.key,
        collateral_pool_withdraw_authority.key,
        destination.key,
        collateral_pool_accounts.token_account.key,
        user_transfer_authority.key,
        amount,
    );

    invoke_signed(
        &ix,
        &[
            collateral_pool_accounts.pool_market,
            collateral_pool_accounts.pool,
            collateral_pool_withdraw_authority,
            destination,
            collateral_pool_accounts.token_account,
            collateral_pool_accounts.pool_market_authority,
            user_transfer_authority,
        ],
        signers_seeds,
    )
}
