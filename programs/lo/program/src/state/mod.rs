//! State types.

mod liquidity_oracle;

use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
pub use liquidity_oracle::*;

/// Enum representing the account type managed by the program
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub enum AccountType {
    /// If the account has not been initialized, the enum will be 0
    Uninitialized,
    /// Random oracle
    LiquidityOracle,
}

impl Default for AccountType {
    fn default() -> Self {
        AccountType::Uninitialized
    }
}
