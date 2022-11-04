//! Depositor state definitions

use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};

mod depositor;
mod internal_mining;
mod rebalancing;
mod rebalancing_step;

pub use depositor::*;
pub use internal_mining::*;
pub use rebalancing::*;
pub use rebalancing_step::*;

/// Enum representing the account type managed by the program
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub enum AccountType {
    /// If the account has not been initialized, the enum will be 0
    Uninitialized,
    /// Depositor
    Depositor,
    /// Rebalancing
    Rebalancing,
    /// Internal mining
    InternalMining,
}

impl Default for AccountType {
    fn default() -> Self {
        AccountType::Uninitialized
    }
}

/// Total rebalancing steps for fixed state
pub const TOTAL_REBALANCING_STEP: usize = 14;
