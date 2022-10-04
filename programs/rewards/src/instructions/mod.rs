//! Program instructions

mod add_vault;
mod claim;
mod deposit_mining;
mod fill_vault;
mod initialize_mining;
mod initialize_pool;
mod initialize_root;
mod migrate_pool;
mod migrate_root;
mod withdraw_mining;

pub use add_vault::*;
pub use claim::*;
pub use deposit_mining::*;
pub use fill_vault::*;
pub use initialize_mining::*;
pub use initialize_pool::*;
pub use initialize_root::*;
pub use migrate_pool::*;
pub use migrate_root::*;
pub use withdraw_mining::*;
