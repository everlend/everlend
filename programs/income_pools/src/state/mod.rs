//! State types
use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};

mod income_pool;
mod income_pool_market;

pub use income_pool::*;
pub use income_pool_market::*;

/// Enum representing the account type managed by the program
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub enum AccountType {
    /// If the account has not been initialized, the enum will be 0
    Uninitialized,
    /// Pool market
    IncomePoolMarket,
    /// Pool
    IncomePool,
}

impl Default for AccountType {
    fn default() -> Self {
        AccountType::Uninitialized
    }
}
