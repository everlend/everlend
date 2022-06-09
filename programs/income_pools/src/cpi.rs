//! CPI

use solana_program::{
    account_info::AccountInfo, program::invoke_signed, program_error::ProgramError,
};

use crate::utils::IncomePoolAccounts;

/// Income pools deposit tokens
#[allow(clippy::too_many_arguments)]
pub fn deposit<'a>(
    income_pool_accounts: IncomePoolAccounts<'a>,
    source: AccountInfo<'a>,
    user_transfer_authority: AccountInfo<'a>,
    amount: u64,
    signers_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    let ix = crate::instruction::deposit(
        &crate::id(),
        income_pool_accounts.pool_market.key,
        income_pool_accounts.pool.key,
        source.key,
        income_pool_accounts.token_account.key,
        user_transfer_authority.key,
        amount,
    );

    invoke_signed(
        &ix,
        &[
            income_pool_accounts.pool_market,
            income_pool_accounts.pool,
            source,
            income_pool_accounts.token_account,
            user_transfer_authority,
        ],
        signers_seeds,
    )
}
