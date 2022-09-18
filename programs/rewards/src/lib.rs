#![deny(missing_docs)]

//! Rewards contract

pub mod state;
pub mod instructions;
pub mod instruction;
pub mod processor;
pub mod cpi;

#[cfg(not(feature = "no-entrypoint"))]
pub mod entrypoint;

pub use solana_program;
use solana_program::pubkey::Pubkey;

solana_program::declare_id!("ELDR7M6m1ysPXks53T7da6zkhnhJV44twXLiAgTf2VpM");

/// Generates mining address
pub fn find_mining_program_address(
    program_id: &Pubkey,
    user: &Pubkey,
    reward_pool: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &["mining".as_bytes(), &user.to_bytes(), &reward_pool.to_bytes()],
        program_id,
    )
}

/// Generates vault address
pub fn find_vault_program_address(
    program_id: &Pubkey,
    reward_pool: &Pubkey,
    reward_mint: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &["vault".as_bytes(), &reward_pool.to_bytes(), &reward_mint.to_bytes()],
        program_id
    )
}

/// Generates reward pool address
pub fn find_reward_pool_program_address(
    program_id: &Pubkey,
    root_account: &Pubkey,
    liquidity_mint: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            "reward_pool".as_bytes(),
            &root_account.to_bytes(),
            &liquidity_mint.to_bytes()
        ],
        program_id
    )
}