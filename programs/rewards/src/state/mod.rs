//! State types

mod mining;
mod reward_pool;
mod rewards_root;
mod deprecated_reward_pool;

use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
pub use mining::*;
pub use reward_pool::*;
pub use rewards_root::*;
pub use deprecated_reward_pool::*;

/// Enum representing the account type managed by the program
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub enum AccountType {
    /// If the account has not been initialized, the enum will be 0
    Uninitialized,
    /// Rewards root
    RewardsRoot,
    /// Reward pool
    RewardPool,
}

impl Default for AccountType {
    fn default() -> Self {
        AccountType::Uninitialized
    }
}