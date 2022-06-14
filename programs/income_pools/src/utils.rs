//! Utils
use solana_program::account_info::AccountInfo;

/// income pool accounts
pub struct IncomePoolAccounts<'a> {
    /// pool market
    pub pool_market: AccountInfo<'a>,
    /// pool
    pub pool: AccountInfo<'a>,
    /// token account
    pub token_account: AccountInfo<'a>,
}
