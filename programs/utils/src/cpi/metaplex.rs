use anchor_lang::Key;
use mpl_token_metadata::{instruction, state::DataV2};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, program::invoke_signed, pubkey::Pubkey,
};

use crate::assert_account_key;

#[allow(clippy::too_many_arguments)]
pub fn create_metadata<'a>(
    program_id: AccountInfo<'a>,
    metadata_account: AccountInfo<'a>,
    pool_mint: AccountInfo<'a>,
    mint_authority: AccountInfo<'a>,
    payer: AccountInfo<'a>,
    _system: AccountInfo<'a>,
    _rent: AccountInfo<'a>,
    name: String,
    symbol: String,
    uri: String,
    signers_seeds: &[&[&[u8]]],
) -> ProgramResult {
    assert_account_key(&program_id, &mpl_token_metadata::id())?;

    // Metadata account
    let (metadata_key, _) = Pubkey::find_program_address(
        &[
            mpl_token_metadata::state::PREFIX.as_bytes(),
            program_id.key.as_ref(),
            pool_mint.key.as_ref(),
        ],
        program_id.key,
    );

    assert_account_key(&metadata_account, &metadata_key)?;

    let ix = instruction::create_metadata_accounts_v2(
        program_id.key(),
        metadata_account.key(),
        pool_mint.key(),
        mint_authority.key(),
        payer.key(),
        mint_authority.key(), // update authority is same as mint - this pool
        name,
        symbol,
        uri,
        None,
        0,
        true,
        true,
        None,
        None,
    );

    invoke_signed(
        &ix,
        &[metadata_account, pool_mint, mint_authority, payer],
        signers_seeds,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn update_metadata<'a>(
    program_id: AccountInfo<'a>,
    metadata_account: AccountInfo<'a>,
    pool_mint: AccountInfo<'a>,
    mint_authority: AccountInfo<'a>,
    name: String,
    symbol: String,
    uri: String,
    signers_seeds: &[&[&[u8]]],
) -> ProgramResult {
    assert_account_key(&program_id, &mpl_token_metadata::id())?;

    // Metadata account
    let (metadata_key, _) = Pubkey::find_program_address(
        &[
            mpl_token_metadata::state::PREFIX.as_bytes(),
            program_id.key.as_ref(),
            pool_mint.key.as_ref(),
        ],
        program_id.key,
    );

    assert_account_key(&metadata_account, &metadata_key)?;

    let ix = instruction::update_metadata_accounts_v2(
        program_id.key(),
        metadata_account.key(),
        mint_authority.key(),
        None,
        Some(DataV2 {
            name,
            symbol,
            uri,
            seller_fee_basis_points: 0,
            creators: None,
            collection: None,
            uses: None,
        }),
        None,
        None,
    );

    invoke_signed(&ix, &[metadata_account, mint_authority], signers_seeds)
}
