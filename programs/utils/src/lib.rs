//! Utils

pub mod accounts;
mod asserts;
mod error;
mod system;
mod token;

pub use asserts::*;
pub use error::*;
pub use system::*;
pub use token::*;

use solana_program::pubkey::Pubkey;

/// Generates seed bump for authorities
pub fn find_program_address(program_id: &Pubkey, pubkey: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[&pubkey.to_bytes()[..32]], program_id)
}
