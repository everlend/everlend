//! State types.

mod currency_distribution;
mod liquidity_oracle;

use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
pub use currency_distribution::*;
pub use liquidity_oracle::*;

/// Enum representing the account type managed by the program
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub enum AccountType {
    /// If the account has not been initialized, the enum will be 0
    Uninitialized,
    /// Liquidity oracle
    LiquidityOracle,
    /// Currency distribution
    CurrencyDistribution,
}

impl Default for AccountType {
    fn default() -> Self {
        AccountType::Uninitialized
    }
}
