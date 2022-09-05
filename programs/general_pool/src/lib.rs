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
use crate::state::ACTUAL_VERSION;
pub use solana_program;
use solana_program::{instruction::AccountMeta, pubkey::Pubkey, system_program, sysvar};

solana_program::declare_id!("GenUMNGcWca1GiPLfg89698Gfys1dzk9BAGsyb9aEL2u");

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
    pool_market_pubkey: &Pubkey,
    token_mint: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            withdrawal_requests_seed().as_bytes(),
            &pool_market_pubkey.to_bytes(),
            &token_mint.to_bytes(),
        ],
        program_id,
    )
}

/// Generates withdrawal requests seed
pub fn withdrawal_requests_seed() -> String {
    let mut withdrawal_requests_seed = "withdrawals".to_owned();
    withdrawal_requests_seed.push_str(&ACTUAL_VERSION.to_string());

    withdrawal_requests_seed
}

/// Generates user withdrawal request address
pub fn find_withdrawal_request_program_address(
    program_id: &Pubkey,
    withdrawal_requests_pubkey: &Pubkey,
    from: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            br"withdrawal",
            &withdrawal_requests_pubkey.to_bytes(),
            &from.to_bytes(),
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
    Pubkey::find_program_address(
        &[br"transit", &pool_market.to_bytes(), &mint.to_bytes()],
        program_id,
    )
}

/// Generate transit unwrap address
pub fn find_transit_sol_unwrap_address(
    program_id: &Pubkey,
    withdrawal_request: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[br"unwrap", &withdrawal_request.to_bytes()], program_id)
}

/// Calculates address of pool config
pub fn find_pool_config_program_address(program_id: &Pubkey, pool: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&["config".as_bytes(), &pool.to_bytes()], program_id)
}

/// Generates user mining address
pub fn find_user_mining_address(
    user: &Pubkey,
    pool_market: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            b"mining".as_ref(),
            user.as_ref(),
            pool_market.as_ref(),
        ],
        &eld_rewards::id(),
    )
}

/// Generate withdraw accounts for SOL mint
pub fn general_pool_withdraw_sol_accounts(
    program_id: &Pubkey,
    general_pool_market: &Pubkey,
    token_mint: &Pubkey,
    from: &Pubkey,
) -> Vec<AccountMeta> {
    let (withdrawal_requests, _) =
        find_withdrawal_requests_program_address(program_id, general_pool_market, token_mint);
    let (withdrawal_request, _) =
        find_withdrawal_request_program_address(program_id, &withdrawal_requests, from);
    let (unwrap_sol_pubkey, _) = find_transit_sol_unwrap_address(program_id, &withdrawal_request);

    vec![
        AccountMeta::new_readonly(*token_mint, false),
        AccountMeta::new(unwrap_sol_pubkey, false),
        AccountMeta::new(*from, true),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
        AccountMeta::new_readonly(system_program::id(), false),
    ]
}
