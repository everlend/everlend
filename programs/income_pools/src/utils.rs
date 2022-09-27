//! Utils
use solana_program::account_info::AccountInfo;

/// income pool accounts
#[derive(Clone, Copy)]
pub struct IncomePoolAccounts<'a, 'b> {
    /// pool market
    pub pool_market: &'a AccountInfo<'b>,
    /// pool
    pub pool: &'a AccountInfo<'b>,
    /// token account
    pub token_account: &'a AccountInfo<'b>,
}
