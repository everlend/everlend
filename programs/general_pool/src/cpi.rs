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
