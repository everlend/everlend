//! State types
use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use std::fmt;

mod pool;
mod pool_borrow_authority;
mod pool_config;
mod pool_market;
mod withdrawal_request;

pub use pool::*;
pub use pool_borrow_authority::*;
pub use pool_config::*;
pub use pool_market::*;
pub use withdrawal_request::*;

/// Enum representing the account type managed by the program
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub enum AccountType {
    /// If the account has not been initialized, the enum will be 0
    Uninitialized,
    /// Pool market
    PoolMarket,
    /// Pool
    Pool,
    /// Pool borrow authority
    PoolBorrowAuthority,
    /// Withdraw requests
    WithdrawRequests,
    /// Withdraw request
    WithdrawRequest,
    /// Pool config
    PoolConfig,
}

impl Default for AccountType {
    fn default() -> Self {
        AccountType::Uninitialized
    }
}

/// Enum representing the account version managed by the program
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub enum AccountVersion {
    /// Default version 0
    V0,
    /// Updated version
    V1,
}

impl Default for AccountVersion {
    fn default() -> Self {
        AccountVersion::V0
    }
}

impl fmt::Display for AccountVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Convert the UI representation of a bp (like 0.5) to the raw bp
pub fn ui_bp_to_bp(ui_ratio: f64) -> u16 {
    (ui_ratio * 10_000f64).round() as u16
}
