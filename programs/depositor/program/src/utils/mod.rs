//! Utils

use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, program_error::ProgramError,
    program_pack::IsInitialized, pubkey::Pubkey,
};

mod mm;
mod ulp;

pub use mm::*;
pub use ulp::*;

use crate::error::DepositorError;

/// Assert unitilialized
pub fn assert_uninitialized<T: IsInitialized>(account: &T) -> ProgramResult {
    if account.is_initialized() {
        Err(ProgramError::AccountAlreadyInitialized)
    } else {
        Ok(())
    }
}

/// Assert owned by
pub fn assert_owned_by(account: &AccountInfo, owner: &Pubkey) -> ProgramResult {
    if account.owner != owner {
        Err(DepositorError::InvalidAccountOwner.into())
    } else {
        Ok(())
    }
}

/// Assert account key
pub fn assert_account_key(account_info: &AccountInfo, key: &Pubkey) -> ProgramResult {
    if *account_info.key != *key {
        Err(ProgramError::InvalidArgument)
    } else {
        Ok(())
    }
}
