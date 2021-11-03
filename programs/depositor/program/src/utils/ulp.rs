use solana_program::{
    account_info::AccountInfo, program::invoke_signed, program_error::ProgramError,
};

/// ULP borrow tokens
#[allow(clippy::too_many_arguments)]
pub fn ulp_borrow<'a>(
    pool_market: AccountInfo<'a>,
    pool: AccountInfo<'a>,
    pool_borrow_authority: AccountInfo<'a>,
    destination: AccountInfo<'a>,
    token_account: AccountInfo<'a>,
    borrow_authority: AccountInfo<'a>,
    amount: u64,
    signers_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    let ix = everlend_ulp::instruction::borrow(
        &everlend_ulp::id(),
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
            destination,
            token_account,
            borrow_authority,
        ],
        signers_seeds,
    )
}
