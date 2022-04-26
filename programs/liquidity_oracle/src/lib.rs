// Export current sdk types for downstream users building with a different sdk version.
pub use solana_program;
use solana_program::pubkey::Pubkey;

pub mod instruction;
pub mod processor;
pub mod state;

#[cfg(not(feature = "no-entrypoint"))]
pub mod entrypoint;

solana_program::declare_id!("LiqNiHY9SnsjQMsfikadZQAsfskBZzzoHZTo3XUeoBV");

/// Generates liquidity oracle token distribution authority address
pub fn find_liquidity_oracle_token_distribution_program_address(
    program_id: &Pubkey,
    liquidity_oracle: &Pubkey,
    token_mint: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            seed().as_bytes(),
            &liquidity_oracle.to_bytes(),
            &token_mint.to_bytes(),
        ],
        program_id,
    )
}

fn seed() -> String {
    let mut seed = "token-distribution".to_string();
    seed.push(state::TokenDistribution::ACTUAL_VERSION as char);
    seed
}

///
pub mod deprecated {
    use super::*;

    ///
    pub fn deprecated_find_liquidity_oracle_token_distribution_program_address(
        program_id: &Pubkey,
        liquidity_oracle: &Pubkey,
        token_mint: &Pubkey,
    ) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[&liquidity_oracle.to_bytes(), &token_mint.to_bytes()],
            program_id,
        )
    }
}
