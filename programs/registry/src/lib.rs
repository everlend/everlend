#![deny(missing_docs)]

//! Registry contract

pub mod instruction;
pub mod processor;
pub mod state;

#[cfg(not(feature = "no-entrypoint"))]
pub mod entrypoint;

// Export current sdk types for downstream users building with a different sdk version
pub use solana_program;
use solana_program::pubkey::Pubkey;

solana_program::declare_id!("EiMuP7K13nrYGwnKB63Fig8uCjJqCpUsY4q5BneRNjcZ");

/// Generates config address
pub fn find_config_program_address(program_id: &Pubkey, registry: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&["config".as_bytes(), &registry.to_bytes()], program_id)
}
