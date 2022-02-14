//! CPI

use solana_program::{
    account_info::AccountInfo, program::invoke_signed, program_error::ProgramError,
};

/// General pool borrow tokens
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

/// General pool repay tokens
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

/// General pool deposit tokens
#[allow(clippy::too_many_arguments)]
pub fn deposit<'a>(
    pool_market: AccountInfo<'a>,
    pool_market_authority: AccountInfo<'a>,
    pool: AccountInfo<'a>,
    source: AccountInfo<'a>,
    destination: AccountInfo<'a>,
    token_account: AccountInfo<'a>,
    pool_mint: AccountInfo<'a>,
    user_transfer_authority: AccountInfo<'a>,
    amount: u64,
    signers_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    let ix = crate::instruction::deposit(
        &crate::id(),
        pool_market.key,
        pool.key,
        source.key,
        destination.key,
        token_account.key,
        pool_mint.key,
        user_transfer_authority.key,
        amount,
    );

    invoke_signed(
        &ix,
        &[
            pool_market,
            pool,
            source,
            destination,
            token_account,
            pool_mint,
            pool_market_authority,
            user_transfer_authority,
        ],
        signers_seeds,
    )
}

/// General pool withdraw tokens
#[allow(clippy::too_many_arguments)]
pub fn withdraw<'a>(
    pool_market: AccountInfo<'a>,
    pool_market_authority: AccountInfo<'a>,
    pool: AccountInfo<'a>,
    withdrawal_requests: AccountInfo<'a>,
    collateral_transit: AccountInfo<'a>,
    destination: AccountInfo<'a>,
    token_account: AccountInfo<'a>,
    token_mint: AccountInfo<'a>,
    pool_mint: AccountInfo<'a>,
    index: u64,
    signers_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    let ix = crate::instruction::withdraw(
        &crate::id(),
        pool_market.key,
        pool.key,
        destination.key,
        token_account.key,
        token_mint.key,
        pool_mint.key,
        index
    );

    invoke_signed(
        &ix,
        &[
            pool_market,
            pool,
            withdrawal_requests,
            destination,
            token_account,
            collateral_transit,
            pool_mint,
            pool_market_authority,
        ],
        signers_seeds,
    )
}
