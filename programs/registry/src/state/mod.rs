//! State types
use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};

mod registry;

pub use registry::*;

/// Enum representing the account type managed by the program
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub enum AccountType {
    /// If the account has not been initialized, the enum will be 0
    Uninitialized,
    /// Registry
    Registry,
}

impl Default for AccountType {
    fn default() -> Self {
        AccountType::Uninitialized
    }
}
