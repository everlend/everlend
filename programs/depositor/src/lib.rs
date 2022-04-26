#![deny(missing_docs)]

//! Depositor contract

// Export current sdk types for downstream users building with a different sdk version
pub use solana_program;
use solana_program::pubkey::Pubkey;

pub mod instruction;
pub mod processor;
pub mod state;
pub mod utils;

#[cfg(not(feature = "no-entrypoint"))]
pub mod entrypoint;

solana_program::declare_id!("DepSR26sqzN67TNf1aZ3VCjTPduzKKqTEY8QQkk3KwEz");

/// Generates transit address
pub fn find_transit_program_address(
    program_id: &Pubkey,
    depositor: &Pubkey,
    mint: &Pubkey,
    seed: &str,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[seed.as_bytes(), &depositor.to_bytes(), &mint.to_bytes()],
        program_id,
    )
}

/// Generates rebalancing address
pub fn find_rebalancing_program_address(
    program_id: &Pubkey,
    depositor: &Pubkey,
    mint: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[seed().as_bytes(), &depositor.to_bytes(), &mint.to_bytes()],
        program_id,
    )
}

fn seed() -> String {
    let mut seed = "rebalancing".to_string();
    seed.push(state::Rebalancing::ACTUAL_VERSION as char);
    seed
}

///
pub mod deprecated {
    use super::*;

    ///
    pub fn deprecated_find_rebalancing_program_address(
        program_id: &Pubkey,
        depositor: &Pubkey,
        mint: &Pubkey,
    ) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[
                "rebalancing".as_bytes(),
                &depositor.to_bytes(),
                &mint.to_bytes(),
            ],
            program_id,
        )
    }
}
