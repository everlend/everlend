#![deny(missing_docs)]

//! Universal liquidity pools contract

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

solana_program::declare_id!("EzDzLfEtcDHfKduQ7pu36rUM2FWfDCLkifp2pzcmGM3p");

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

/// Generates withdrawal requests address
pub fn find_withdrawal_requests_program_address(
    program_id: &Pubkey,
    pool_pubkey: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            "withdrawals".as_bytes(),
            &pool_pubkey.to_bytes(),
        ],
        program_id,
    )
}

/// Generates transit address
pub fn find_transit_program_address(
    program_id: &Pubkey,
    pool_market: &Pubkey,
    mint: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(&["transit".as_bytes(), &pool_market.to_bytes(), &mint.to_bytes()], program_id)
}
