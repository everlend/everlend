//! Pool market state definitions
use super::*;
use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use everlend_utils::{AccountVersion, Uninitialized};
use solana_program::{
    msg,
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};

/// Pool market
#[repr(C)]
#[derive(Debug, BorshDeserialize, BorshSerialize, BorshSchema, Default)]
pub struct PoolMarket {
    /// Account type - PoolMarket
    pub account_type: AccountType,
    /// Account version
    pub account_version: AccountVersion,
    /// Market manager
    pub manager: Pubkey,
    /// Registry
    pub registry: Pubkey,
}

impl PoolMarket {
    /// Account actual version
    const ACTUAL_VERSION: AccountVersion = AccountVersion::V0;
    /// Initialize a Pool market
    pub fn init(&mut self, params: InitPoolMarketParams) {
        self.account_type = AccountType::PoolMarket;
        self.account_version = Self::ACTUAL_VERSION;
        self.manager = params.manager;
        self.registry = params.registry;
    }
}

/// Initialize a Pool market params
pub struct InitPoolMarketParams {
    /// Market manager
    pub manager: Pubkey,
    /// Registry
    pub registry: Pubkey,
}

impl Sealed for PoolMarket {}
impl Pack for PoolMarket {
    // 1 + 1 + 32 + 32
    const LEN: usize = 66;

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let mut slice = dst;
        self.serialize(&mut slice).unwrap()
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, solana_program::program_error::ProgramError> {
        Self::try_from_slice(src).map_err(|_| {
            msg!("Failed to deserialize");
            msg!("Actual LEN: {}", std::mem::size_of::<PoolMarket>());
            ProgramError::InvalidAccountData
        })
    }
}

impl IsInitialized for PoolMarket {
    fn is_initialized(&self) -> bool {
        self.account_type == AccountType::PoolMarket
    }
}

impl Uninitialized for PoolMarket {
    fn is_uninitialized(&self) -> bool {
        self.account_type == AccountType::default()
    }
}
