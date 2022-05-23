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
        &[
            rebalancing_seed().as_bytes(),
            &depositor.to_bytes(),
            &mint.to_bytes(),
        ],
        program_id,
    )
}

/// Generates rebalancing seed
pub fn rebalancing_seed() -> String {
    String::from("rebalancing")
}

/// Generates internal mining address
pub fn find_internal_mining_program_address(
    program_id: &Pubkey,
    collateral_mint: &Pubkey,
    money_market_program_id: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            internal_mining_seed().as_bytes(),
            &collateral_mint.to_bytes(),
            &money_market_program_id.to_bytes(),
        ],
        program_id,
    )
}

/// Generates internal mining seed
pub fn internal_mining_seed() -> String {
    let mut withdrawal_requests_seed = "internal_mining".to_owned();
    // withdrawal_requests_seed.push_str(&ACTUAL_VERSION.to_string());

    withdrawal_requests_seed
}
