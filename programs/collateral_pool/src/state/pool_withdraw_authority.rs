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

/// Pool
#[repr(C)]
#[derive(Debug, BorshDeserialize, BorshSerialize, BorshSchema, Default)]
pub struct PoolWithdrawAuthority {
    /// Account type - PoolWithdrawAuthority
    pub account_type: AccountType,
    /// Pool
    pub pool: Pubkey,
    /// Withdraw authority
    pub withdraw_authority: Pubkey,
    /// Amount withdrawn
    pub amount_withdrawn: u64,
}

impl PoolWithdrawAuthority {
    /// Initialize a PoolWithdrawAuthority
    pub fn init(&mut self, params: InitPoolWithdrawAuthorityParams) {
        self.account_type = AccountType::PoolWithdrawAuthority;
        self.pool = params.pool;
        self.withdraw_authority = params.withdraw_authority;
        self.amount_withdrawn = 0;
    }

    /// Withdraw collateral
    pub fn withdraw(&mut self, amount: u64) -> ProgramResult {
        self.amount_withdrawn = self
            .amount_withdrawn
            .checked_add(amount)
            .ok_or(EverlendError::MathOverflow)?;
        Ok(())
    }
}

/// Initialize a PoolWithdrawAuthority params
pub struct InitPoolWithdrawAuthorityParams {
    /// Pool
    pub pool: Pubkey,
    /// Withdraw authority
    pub withdraw_authority: Pubkey,
}

impl Sealed for PoolWithdrawAuthority {}
impl Pack for PoolWithdrawAuthority {
    // 1 + 32 + 32 + 8
    const LEN: usize = 73;

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

impl IsInitialized for PoolWithdrawAuthority {
    fn is_initialized(&self) -> bool {
            self.account_type == AccountType::PoolWithdrawAuthority
    }
}

