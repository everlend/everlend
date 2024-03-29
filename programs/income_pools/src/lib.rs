#![deny(missing_docs)]

//! Income pools contract

pub mod cpi;
pub mod instruction;
pub mod processor;
pub mod state;
pub mod utils;

#[cfg(not(feature = "no-entrypoint"))]
pub mod entrypoint;

// Export current sdk types for downstream users building with a different sdk version
pub use solana_program;
use solana_program::pubkey::Pubkey;

solana_program::declare_id!("incmgBdqLbD6qmxn3Ru7Dbbm1UiAMrvkhwgSdUz5EYX");

/// Generates pool address
pub fn find_pool_program_address(
    program_id: &Pubkey,
    pool_market_pubkey: &Pubkey,
    token_mint_pubkey: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            &pool_market_pubkey.to_bytes(),
            &token_mint_pubkey.to_bytes(),
        ],
        program_id,
    )
}

/// Generates safety fund token account address
pub fn find_safety_fund_token_account_address(
    program_id: &Pubkey,
    pool_market_pubkey: &Pubkey,
    token_mint_pubkey: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            safety_fund_token_account_seed().as_bytes(),
            &pool_market_pubkey.to_bytes(),
            &token_mint_pubkey.to_bytes(),
        ],
        program_id,
    )
}

/// Generate safety fund token account seed
pub fn safety_fund_token_account_seed() -> String {
    "safety_fund".to_owned()
}
