//! Pool state definitions

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
pub struct Pool {
    /// Account type - Pool
    pub account_type: AccountType,
    /// Pool market
    pub pool_market: Pubkey,
    /// Token mint
    pub token_mint: Pubkey,
    /// Token account
    pub token_account: Pubkey,
    /// Total amount borrowed
    pub total_amount_borrowed: u64,
}

impl Pool {
    /// Initialize a Pool
    pub fn init(&mut self, params: InitPoolParams) {
        self.account_type = AccountType::Pool;
        self.pool_market = params.pool_market;
        self.token_mint = params.token_mint;
        self.token_account = params.token_account;
        self.total_amount_borrowed = 0;
    }

    /// Borrow funds
    pub fn borrow(&mut self, amount: u64) -> ProgramResult {
        self.total_amount_borrowed = self
            .total_amount_borrowed
            .checked_add(amount)
            .ok_or(EverlendError::MathOverflow)?;
        Ok(())
    }

    /// Repay funds
    pub fn repay(&mut self, amount: u64) -> ProgramResult {
        if self.total_amount_borrowed.lt(&amount) {
            return Err(EverlendError::RepayAmountCheckFailed.into());
        }

        self.total_amount_borrowed = self
            .total_amount_borrowed
            .checked_sub(amount)
            .ok_or(EverlendError::MathOverflow)?;
        Ok(())
    }
}

/// Initialize a Pool params
pub struct InitPoolParams {
    /// Pool market
    pub pool_market: Pubkey,
    /// Token mint
    pub token_mint: Pubkey,
    /// Token account
    pub token_account: Pubkey,
}

impl Sealed for Pool {}
impl Pack for Pool {
    // 1 + 32 + 32 + 32 + 32 + 8
    const LEN: usize = 137;

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let mut slice = dst;
        self.serialize(&mut slice).unwrap()
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, solana_program::program_error::ProgramError> {
        Self::try_from_slice(src).map_err(|_| {
            msg!("Failed to deserialize");
            msg!("Actual LEN: {}", std::mem::size_of::<Pool>());
            ProgramError::InvalidAccountData
        })
    }
}

impl IsInitialized for Pool {
    fn is_initialized(&self) -> bool {
        self.account_type != AccountType::Uninitialized && self.account_type == AccountType::Pool
    }
}
