//! State types.

mod liquidity_oracle;
mod token_distribution_deprecated;
mod token_oracle;

use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
pub use liquidity_oracle::*;
pub use token_distribution_deprecated::*;
pub use token_oracle::*;

/// Enum representing the account type managed by the program
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub enum AccountType {
    /// If the account has not been initialized, the enum will be 0
    Uninitialized,
    /// Root liquidity oracle account
    LiquidityOracle,
    /// Pool oracle
    TokenOracle,
}

impl Default for AccountType {
    fn default() -> Self {
        AccountType::Uninitialized
    }
}
