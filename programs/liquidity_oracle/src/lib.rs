pub mod instruction;
pub mod instructions;
pub mod processor;
pub mod state;

#[cfg(not(feature = "no-entrypoint"))]
pub mod entrypoint;

// Export current sdk types for downstream users building with a different sdk version.
pub use solana_program;
use solana_program::pubkey::Pubkey;

solana_program::declare_id!("LiqNiHY9SnsjQMsfikadZQAsfskBZzzoHZTo3XUeoBV");

/// Generates liquidity oracle token distribution authority address
pub fn find_token_distribution_program_address(
    program_id: &Pubkey,
    liquidity_oracle: &Pubkey,
    token_mint: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[&liquidity_oracle.to_bytes(), &token_mint.to_bytes()],
        program_id,
    )
}
