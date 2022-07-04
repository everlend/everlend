//! Income pool state definitions

use super::*;
use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use everlend_utils::UnInitialized;
use solana_program::{
    msg,
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};

/// Income pool
#[repr(C)]
#[derive(Debug, BorshDeserialize, BorshSerialize, BorshSchema, Default)]
pub struct IncomePool {
    /// Account type - IncomePool
    pub account_type: AccountType,
    /// Income pool market
    pub income_pool_market: Pubkey,
    /// Token mint
    pub token_mint: Pubkey,
    /// Token account
    pub token_account: Pubkey,
}

impl IncomePool {
    /// Initialize a income pool
    pub fn init(&mut self, params: InitIncomePoolParams) {
        self.account_type = AccountType::IncomePool;
        self.income_pool_market = params.income_pool_market;
        self.token_mint = params.token_mint;
        self.token_account = params.token_account;
    }
}

/// Initialize a income pool params
pub struct InitIncomePoolParams {
    /// Income pool market
    pub income_pool_market: Pubkey,
    /// Token mint
    pub token_mint: Pubkey,
    /// Token account
    pub token_account: Pubkey,
}

impl Sealed for IncomePool {}
impl Pack for IncomePool {
    // 1 + 32 + 32 + 32
    const LEN: usize = 97;

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let mut slice = dst;
        self.serialize(&mut slice).unwrap()
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, solana_program::program_error::ProgramError> {
        Self::try_from_slice(src).map_err(|_| {
            msg!("Failed to deserialize");
            msg!("Actual LEN: {}", std::mem::size_of::<IncomePool>());
            ProgramError::InvalidAccountData
        })
    }
}

impl IsInitialized for IncomePool {
    fn is_initialized(&self) -> bool {
        self.account_type != AccountType::Uninitialized
            && self.account_type == AccountType::IncomePool
    }
}

impl UnInitialized for IncomePool {
    fn is_uninitialized(&self) -> bool {
        self.account_type == AccountType::default()
    }
}
