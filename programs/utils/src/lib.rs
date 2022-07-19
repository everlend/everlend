//! Utils

mod asserts;
pub mod cpi;
mod error;
pub mod instructions;
pub mod integrations;
pub mod math;

pub use asserts::*;
pub use error::*;
pub use math::*;

use sha2::{Digest, Sha256};
use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::pubkey::Pubkey;

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


pub struct AnchorInstruction;

impl AnchorInstruction {
    /// Create `AnchorInstruction`.
    ///
    pub fn new(name: &[u8]) -> Vec<u8>{
        let mut hasher = Sha256::new();
        hasher.update([b"global:", name].concat());
        hasher.finalize()[..8].to_vec()
    }

    pub fn new_with_data<T: BorshSerialize>(name: &[u8], data: &T) -> Vec<u8>{
        let data = data.try_to_vec().unwrap();
        let mut hasher = Sha256::new();
        hasher.update([b"global:", name].concat());
        let ix = &hasher.finalize()[..8];

        [ix, &data[..]].concat()
    }
}