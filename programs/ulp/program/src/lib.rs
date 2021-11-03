#![deny(missing_docs)]

//! Universal liquidity pools contract

pub mod error;
pub mod instruction;
pub mod processor;
pub mod state;
pub mod utils;

#[cfg(not(feature = "no-entrypoint"))]
pub mod entrypoint;

// Export current sdk types for downstream users building with a different sdk version
pub use solana_program;
use solana_program::pubkey::Pubkey;

solana_program::declare_id!("sFPqhpo9CJ4sCMPwsaZwmC25WERMW27x1M1be3DY5BM");

/// Generates seed bump for authorities
pub fn find_program_address(program_id: &Pubkey, pubkey: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[&pubkey.to_bytes()[..32]], program_id)
}

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

/// Generates pool borrow authority address
pub fn find_pool_borrow_authority_program_address(
    program_id: &Pubkey,
    pool_pubkey: &Pubkey,
    borrow_authority: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[&pool_pubkey.to_bytes(), &borrow_authority.to_bytes()],
        program_id,
    )
}
