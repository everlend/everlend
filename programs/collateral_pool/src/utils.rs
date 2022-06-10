//! Utils

use everlend_utils::EverlendError;
use solana_program::{account_info::AccountInfo, program_error::ProgramError, program_pack::Pack};
use spl_token::state::Account;

/// Collateral pool accounts
pub struct CollateralPoolAccounts<'a> {
    /// pool market
    pub pool_market: AccountInfo<'a>,
    /// pool market authority
    pub pool_market_authority: AccountInfo<'a>,
    /// pool
    pub pool: AccountInfo<'a>,
    /// token account
    pub token_account: AccountInfo<'a>,
}

/// Get total pool amount
pub fn total_pool_amount(
    token_account: AccountInfo,
    total_amount_borrowed: u64,
) -> Result<u64, ProgramError> {
    let token_amount = Account::unpack_unchecked(&token_account.data.borrow())?.amount;
    Ok(token_amount
        .checked_add(total_amount_borrowed)
        .ok_or(EverlendError::MathOverflow)?)
}
