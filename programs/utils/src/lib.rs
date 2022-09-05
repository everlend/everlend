//! Utils

mod asserts;
pub mod cpi;
mod error;
pub mod instructions;
pub mod integrations;
pub mod math;

pub use asserts::*;
pub use error::*;
pub use math::*;

use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::pubkey::Pubkey;
use solana_program::{account_info::AccountInfo, program_error::ProgramError};
/// Generates seed bump for authorities
pub fn find_program_address(program_id: &Pubkey, pubkey: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[&pubkey.to_bytes()[..32]], program_id)
}

/// Enum representing the account version managed by the program
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub enum AccountVersion {
    /// Default version 0
    V0,
    /// Updated version
    V1,
}

impl Default for AccountVersion {
    fn default() -> Self {
        AccountVersion::V0
    }
}

pub fn next_uninitialized_account<'a, 'b, I: Iterator<Item = &'a AccountInfo<'b>>>(
    iter: &mut I,
) -> Result<I::Item, ProgramError> {
    let acc = iter.next().ok_or(ProgramError::NotEnoughAccountKeys)?;
    if acc.owner.eq(&Pubkey::default()) {
        Ok(acc)
    } else {
        Err(ProgramError::AccountAlreadyInitialized)
    }
}

pub fn next_account<'a, 'b, I: Iterator<Item = &'a AccountInfo<'b>>>(
    iter: &mut I,
    owner: &Pubkey,
) -> Result<I::Item, ProgramError> {
    let acc = iter.next().ok_or(ProgramError::NotEnoughAccountKeys)?;
    assert_owned_by(acc, owner)?;

    Ok(acc)
}

pub fn next_program_account<'a, 'b, I: Iterator<Item = &'a AccountInfo<'b>>>(
    iter: &mut I,
    key: &Pubkey,
) -> Result<I::Item, ProgramError> {
    let acc = iter.next().ok_or(ProgramError::NotEnoughAccountKeys)?;
    assert_account_key(acc, key)?;

    Ok(acc)
}

pub fn next_signer_account<'a, 'b, I: Iterator<Item = &'a AccountInfo<'b>>>(
    iter: &mut I,
) -> Result<I::Item, ProgramError> {
    let acc = iter.next().ok_or(ProgramError::NotEnoughAccountKeys)?;
    assert_signer(acc)?;

    Ok(acc)
}
