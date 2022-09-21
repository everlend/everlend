//! Utils

mod asserts;
pub mod cpi;
mod error;
pub mod instructions;
pub mod integrations;
pub mod math;

use std::iter::Enumerate;

pub use asserts::*;
pub use error::*;
pub use math::*;

use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::{account_info::AccountInfo, msg, program_error::ProgramError, pubkey::Pubkey};
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

pub struct AccountLoader {}

impl AccountLoader {
    /// Checks that account is not initilized (it's pubkey is empty)
    pub fn next_uninitialized<'a, 'b, I: Iterator<Item = &'a AccountInfo<'b>>>(
        iter: &mut Enumerate<I>,
    ) -> Result<I::Item, ProgramError> {
        let (idx, acc) = iter.next().ok_or(ProgramError::NotEnoughAccountKeys)?;
        if acc.owner.eq(&Pubkey::default()) {
            return Ok(acc);
        }

        msg!("Account #{}:{} already initialized", idx, acc.key,);
        Err(ProgramError::AccountAlreadyInitialized)
    }

    pub fn next_with_owner<'a, 'b, I: Iterator<Item = &'a AccountInfo<'b>>>(
        iter: &mut Enumerate<I>,
        owner: &Pubkey,
    ) -> Result<I::Item, ProgramError> {
        let (idx, acc) = iter.next().ok_or(ProgramError::NotEnoughAccountKeys)?;
        if acc.owner.eq(owner) {
            return Ok(acc);
        }

        msg!(
            "Account #{}:{} owner error. Got {} Expected {}",
            idx,
            acc.key,
            acc.owner,
            owner
        );
        Err(EverlendError::InvalidAccountOwner.into())
    }

    pub fn next_with_key<'a, 'b, I: Iterator<Item = &'a AccountInfo<'b>>>(
        iter: &mut Enumerate<I>,
        key: &Pubkey,
    ) -> Result<I::Item, ProgramError> {
        let (idx, acc) = iter.next().ok_or(ProgramError::NotEnoughAccountKeys)?;
        if acc.key.eq(key) {
            return Ok(acc);
        }

        msg!(
            "Account #{}:{} assert error. Expected {}",
            idx,
            acc.key,
            key
        );
        Err(ProgramError::InvalidArgument)
    }

    pub fn next_signer<'a, 'b, I: Iterator<Item = &'a AccountInfo<'b>>>(
        iter: &mut Enumerate<I>,
    ) -> Result<I::Item, ProgramError> {
        let (idx, acc) = iter.next().ok_or(ProgramError::NotEnoughAccountKeys)?;
        if acc.is_signer {
            return Ok(acc);
        }

        msg!("Account #{}:{} missing signature", idx, acc.key,);
        Err(ProgramError::MissingRequiredSignature)
    }

    /// Checks if account is initialized and then checks it's owner
    pub fn next_optional<'a, 'b, I: Iterator<Item = &'a AccountInfo<'b>>>(
        iter: &mut Enumerate<I>,
        owner: &Pubkey,
    ) -> Result<I::Item, ProgramError> {
        let (idx, acc) = iter.next().ok_or(ProgramError::NotEnoughAccountKeys)?;
        if acc.owner.eq(&Pubkey::default()) {
            return Ok(acc);
        }

        if acc.owner.eq(owner) {
            return Ok(acc);
        }

        msg!(
            "Account #{}:{} owner error. Got {} Expected unitialized or {}",
            idx,
            acc.key,
            acc.owner,
            owner
        );
        Err(EverlendError::InvalidAccountOwner.into())
    }

    /// Load the account without any checks
    pub fn next_unchecked<'a, 'b, I: Iterator<Item = &'a AccountInfo<'b>>>(
        iter: &mut Enumerate<I>,
    ) -> Result<I::Item, ProgramError> {
        let (_, acc) = iter.next().ok_or(ProgramError::NotEnoughAccountKeys)?;
        Ok(acc)
    }

    pub fn has_more<I: Iterator>(iter: &Enumerate<I>) -> bool {
        let (remaining_len, _) = iter.size_hint();
        remaining_len > 0
    }
}
