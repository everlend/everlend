#![deny(missing_docs)]

//! Registry contract

// Export current sdk types for downstream users building with a different sdk version
pub use solana_program;
use solana_program::pubkey::Pubkey;

pub mod instruction;
pub mod processor;
pub mod state;
// pub use seed as seed_config_program_address;

#[cfg(not(feature = "no-entrypoint"))]
pub mod entrypoint;

solana_program::declare_id!("RegYdXL5fJF247zmeLSXXiUPjhpn4TMYLr94QRqkN8P");

/// Generates config address
pub fn find_config_program_address(program_id: &Pubkey, registry: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[seed().as_bytes(), &registry.to_bytes()], program_id)
}

///
fn seed() -> String {
    let mut seed = "config".to_string();
    seed.push(state::RegistryConfig::ACTUAL_VERSION as char);
    seed
}

///
pub mod deprecated {
    use super::*;

    ///
    pub fn deprecated_find_config_program_address(
        program_id: &Pubkey,
        registry: &Pubkey,
    ) -> (Pubkey, u8) {
        Pubkey::find_program_address(&["config".as_bytes(), &registry.to_bytes()], program_id)
    }
}
