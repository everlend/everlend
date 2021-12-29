//! CPI

use solana_program::{
    account_info::AccountInfo, program::invoke_signed, program_error::ProgramError,
};

/// Income pools deposit tokens
#[allow(clippy::too_many_arguments)]
pub fn deposit<'a>(
    pool_market: AccountInfo<'a>,
    pool_market_authority: AccountInfo<'a>,
    pool: AccountInfo<'a>,
    source: AccountInfo<'a>,
    token_account: AccountInfo<'a>,
    user_transfer_authority: AccountInfo<'a>,
    amount: u64,
    signers_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    let ix = crate::instruction::deposit(
        &crate::id(),
        pool_market.key,
        pool.key,
        source.key,
        token_account.key,
        user_transfer_authority.key,
        amount,
    );

    invoke_signed(
        &ix,
        &[
            pool_market,
            pool,
            source,
            token_account,
            pool_market_authority,
            user_transfer_authority,
        ],
        signers_seeds,
    )
}
