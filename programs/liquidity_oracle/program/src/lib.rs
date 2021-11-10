pub mod instruction;
pub mod processor;
pub mod state;
mod utils;

#[cfg(not(feature = "no-entrypoint"))]
pub mod entrypoint;
pub mod error;

// Export current sdk types for downstream users building with a different sdk version.
pub use solana_program;
use solana_program::pubkey::Pubkey;

solana_program::declare_id!("FfYvEMJip3kLpSJKfyLRXhp8f8yuSSaLxtjzaFecLT9s");

/// Generates liquidity oracle currency distribution authority address
pub fn find_liquidity_oracle_currency_distribution_program_address(
    program_id: &Pubkey,
    liquidity_oracle: &Pubkey,
    currency: &String,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[&liquidity_oracle.to_bytes(), currency.as_bytes()],
        program_id,
    )
}
