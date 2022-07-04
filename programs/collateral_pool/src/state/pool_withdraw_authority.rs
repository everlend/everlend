//! PoolBorrowAuthority state definitions
use super::*;
use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use everlend_utils::UnInitialized;
use solana_program::{
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
}

impl PoolWithdrawAuthority {
    /// Initialize a PoolWithdrawAuthority
    pub fn init(&mut self, pool: Pubkey, withdraw_authority: Pubkey) {
        self.account_type = AccountType::PoolWithdrawAuthority;
        self.pool = pool;
        self.withdraw_authority = withdraw_authority;
    }
}

impl Sealed for PoolWithdrawAuthority {}
impl Pack for PoolWithdrawAuthority {
    // 1 + 32 + 32
    const LEN: usize = 65;

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

impl UnInitialized for PoolWithdrawAuthority {
    fn is_uninitialized(&self) -> bool {
        self.account_type == AccountType::default()
    }
}
