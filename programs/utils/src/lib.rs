//! Utils

use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::pubkey::Pubkey;

pub use asserts::*;
pub use error::*;
pub use math::*;

mod asserts;
pub mod cpi;
mod error;
pub mod integrations;
pub mod math;

/// Generates seed bump for authorities
pub fn find_program_address(program_id: &Pubkey, pubkey: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[&pubkey.to_bytes()[..32]], program_id)
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
