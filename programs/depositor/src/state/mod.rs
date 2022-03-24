//! Depositor state definitions

use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};

mod depositor;
mod rebalancing;
mod rebalancing_step;

pub use depositor::*;
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
}

impl Default for AccountType {
    fn default() -> Self {
        AccountType::Uninitialized
    }
}

/// Total rebalancing steps for fixed state
// TODO: Change to more
pub const TOTAL_REBALANCING_STEP: usize = 4;
