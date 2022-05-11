//! State types
use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};

mod registry;
mod registry_config;
mod pool_config;

pub use registry::*;
pub use registry_config::*;
pub use pool_config::*;

/// Enum representing the account type managed by the program
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub enum AccountType {
    /// If the account has not been initialized, the enum will be 0
    Uninitialized,
    /// Registry
    Registry,
    /// RegistryConfig
    RegistryConfig,
    /// RegistryPoolConfig
    RegistryPoolConfig,
}

impl Default for AccountType {
    fn default() -> Self {
        AccountType::Uninitialized
    }
}
