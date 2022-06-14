//! State types
use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};

mod pool;
mod pool_borrow_authority;
mod pool_withdraw_authority;
mod pool_market;

pub use pool::*;
pub use pool_borrow_authority::*;
pub use pool_withdraw_authority::*;
pub use pool_market::*;

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
    /// Pool withdraw authority
    PoolWithdrawAuthority,
}

impl Default for AccountType {
    fn default() -> Self {
        AccountType::Uninitialized
    }
}

/// Convert the UI representation of a bp (like 0.5) to the raw bp
pub fn ui_bp_to_bp(ui_ratio: f64) -> u16 {
    (ui_ratio * 10_000f64).round() as u16
}
