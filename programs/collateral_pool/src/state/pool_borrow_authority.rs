//! PoolBorrowAuthority state definitions
use super::*;
use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use everlend_utils::EverlendError;
use solana_program::{
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};

/// Pool borrow authority
#[repr(C)]
#[derive(Debug, BorshDeserialize, BorshSerialize, BorshSchema, Default)]
pub struct PoolBorrowAuthority {
    /// Account type - PoolBorrowAuthority
    pub account_type: AccountType,
    /// Pool
    pub pool: Pubkey,
    /// Borrow authority
    pub borrow_authority: Pubkey,
    /// Amount borrowed
    pub amount_borrowed: u64,
    /// Share allowed
    pub share_allowed: u16,
}

impl PoolBorrowAuthority {
    /// Initialize a PoolBorrowAuthority
    pub fn init(&mut self, params: InitPoolBorrowAuthorityParams) {
        self.account_type = AccountType::PoolBorrowAuthority;
        self.pool = params.pool;
        self.borrow_authority = params.borrow_authority;
        self.amount_borrowed = 0;
        self.share_allowed = params.share_allowed;
    }

    /// Borrow funds
    pub fn borrow(&mut self, amount: u64) -> ProgramResult {
        self.amount_borrowed = self
            .amount_borrowed
            .checked_add(amount)
            .ok_or(EverlendError::MathOverflow)?;
        Ok(())
    }

    /// Repay funds
    pub fn repay(&mut self, amount: u64) -> ProgramResult {
        if self.amount_borrowed.lt(&amount) {
            return Err(EverlendError::RepayAmountCheckFailed.into());
        }

        self.amount_borrowed = self
            .amount_borrowed
            .checked_sub(amount)
            .ok_or(EverlendError::MathOverflow)?;
        Ok(())
    }

    /// Update share allowed
    pub fn update_share_allowed(&mut self, share: u16) {
        self.share_allowed = share
    }

    /// Get amount allowed
    pub fn get_amount_allowed(&self, total_pool_amount: u64) -> Result<u64, ProgramError> {
        Ok((total_pool_amount as u128)
            .checked_mul(self.share_allowed as u128)
            .ok_or(EverlendError::MathOverflow)?
            .checked_div(10_000)
            .ok_or(EverlendError::MathOverflow)? as u64)
    }

    /// Check amount allowed
    pub fn check_amount_allowed(&self, total_pool_amount: u64) -> ProgramResult {
        if self.amount_borrowed > self.get_amount_allowed(total_pool_amount)? {
            Err(EverlendError::AmountAllowedCheckFailed.into())
        } else {
            Ok(())
        }
    }
}

/// Initialize a PoolBorrowAuthority params
pub struct InitPoolBorrowAuthorityParams {
    /// Pool
    pub pool: Pubkey,
    /// Borrow authority
    pub borrow_authority: Pubkey,
    /// Share allowed
    pub share_allowed: u16,
}

impl Sealed for PoolBorrowAuthority {}
impl Pack for PoolBorrowAuthority {
    // 1 + 32 + 32 + 8 + 2
    const LEN: usize = 75;

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let mut slice = dst;
        self.serialize(&mut slice).unwrap()
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, solana_program::program_error::ProgramError> {
        Self::try_from_slice(src).map_err(|_| {
            msg!("Failed to deserialize");
            msg!("Actual LEN: {}", std::mem::size_of::<PoolBorrowAuthority>());
            ProgramError::InvalidAccountData
        })
    }
}

impl IsInitialized for PoolBorrowAuthority {
    fn is_initialized(&self) -> bool {
        self.account_type != AccountType::Uninitialized
            && self.account_type == AccountType::PoolBorrowAuthority
    }
}
