//! CPI

use solana_program::{
    account_info::AccountInfo, program::invoke_signed, program_error::ProgramError, pubkey::Pubkey,
    entrypoint::ProgramResult,
};
use solana_program::instruction::AccountMeta;
use crate::instruction;

///Rewards deposit mining
#[allow(clippy::too_many_arguments)]
pub fn deposit_money_market<'a>(
    program_id: &Pubkey,
    liquidity_mint: AccountInfo<'a>,
    collateral_transit: AccountInfo<'a>,
    collateral_mint: AccountInfo<'a>,
    deposit_authority: AccountInfo<'a>,
    mm_program_id: AccountInfo<'a>,
    mm_accounts: Vec<AccountInfo>,
    mm_index: u8,
    amount: u64,
    signers_seeds: &[&[&[u8]]],
) -> ProgramResult {
    // x.get_name()
    let mm_accounts_meta = mm_accounts.iter().
        map(|x|  AccountMeta{ pubkey: *x.key, is_signer: x.is_signer, is_writable: x.is_writable} )
        .collect();

    let ix = instruction::deposit_money_market(
        program_id,
        deposit_authority.key,
        liquidity_mint.key,
        collateral_transit.key,
        collateral_mint.key,
        mm_program_id.key,
        mm_accounts_meta,
        mm_index,
        amount,
    );

    invoke_signed(
        &ix,
        &[deposit_authority, liquidity_mint, collateral_transit, collateral_mint, mm_program_id],
        signers_seeds,
    )
}
