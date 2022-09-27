//! CPI

use solana_program::{
    account_info::AccountInfo, program::invoke_signed, program_error::ProgramError,
};

use crate::utils::IncomePoolAccounts;

/// Income pools deposit tokens
#[allow(clippy::too_many_arguments)]
pub fn deposit<'a, 'b>(
    income_pool_accounts: IncomePoolAccounts<'a, 'b>,
    source: AccountInfo<'b>,
    user_transfer_authority: AccountInfo<'b>,
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
            income_pool_accounts.pool_market.clone(),
            income_pool_accounts.pool.clone(),
            source,
            income_pool_accounts.token_account.clone(),
            user_transfer_authority,
        ],
        signers_seeds,
    )
}
