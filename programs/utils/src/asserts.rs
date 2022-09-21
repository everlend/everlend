use crate::EverlendError;
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    program_pack::IsInitialized, pubkey::Pubkey, rent::Rent,
};

pub trait Uninitialized {
    /// Is uninitialized
    fn is_uninitialized(&self) -> bool;
}

/// Assert signer.
pub fn assert_signer(account: &AccountInfo) -> ProgramResult {
    if account.is_signer {
        return Ok(());
    }

    Err(ProgramError::MissingRequiredSignature)
}

/// Assert initilialized
pub fn assert_initialized<T: IsInitialized>(account: &T) -> ProgramResult {
    if account.is_initialized() {
        Ok(())
    } else {
        Err(ProgramError::UninitializedAccount)
    }
}

/// Assert unitilialized
pub fn assert_uninitialized<T: Uninitialized>(account: &T) -> ProgramResult {
    if account.is_uninitialized() {
        Ok(())
    } else {
        Err(ProgramError::AccountAlreadyInitialized)
    }
}

/// Assert owned by
pub fn assert_owned_by(account: &AccountInfo, owner: &Pubkey) -> ProgramResult {
    if account.owner != owner {
        msg!(
            "Assert {} owner error. Got {} Expected {}",
            *account.key,
            *account.owner,
            *owner
        );
        Err(EverlendError::InvalidAccountOwner.into())
    } else {
        Ok(())
    }
}

/// Assert account key
pub fn assert_account_key(account_info: &AccountInfo, key: &Pubkey) -> ProgramResult {
    if *account_info.key != *key {
        msg!(
            "Assert account error. Got {} Expected {}",
            *account_info.key,
            *key
        );
        Err(ProgramError::InvalidArgument)
    } else {
        Ok(())
    }
}

/// Assert rent exempt
pub fn assert_rent_exempt(rent: &Rent, account_info: &AccountInfo) -> ProgramResult {
    if !rent.is_exempt(account_info.lamports(), account_info.data_len()) {
        msg!(&rent.minimum_balance(account_info.data_len()).to_string());
        Err(ProgramError::AccountNotRentExempt)
    } else {
        Ok(())
    }
}

/// Assert zero amount
pub fn assert_zero_amount(amount: u64) -> ProgramResult {
    if amount == 0 {
        return Err(EverlendError::ZeroAmount.into())
    }

    Ok(())
}