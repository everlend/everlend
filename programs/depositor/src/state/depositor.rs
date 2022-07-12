//! Program state definitions

use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use everlend_utils::{AccountVersion, EverlendError, Uninitialized};
use solana_program::{
    msg,
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};

use super::AccountType;

/// Depositor
#[repr(C)]
#[derive(Debug, BorshDeserialize, BorshSerialize, BorshSchema, Default)]
pub struct Depositor {
    /// Account type - Depositor
    pub account_type: AccountType,
    /// Account version
    pub account_version: AccountVersion,
    /// Registry
    pub registry: Pubkey,
    /// Rebalance executor
    pub rebalance_executor: Pubkey,
}

impl Depositor {
    /// Account actual version
    const ACTUAL_VERSION: AccountVersion = AccountVersion::V0;

    /// Initialize a voting pool
    pub fn init(&mut self, params: InitDepositorParams) {
        self.account_type = AccountType::Depositor;
        self.registry = params.registry;
        self.account_version = Self::ACTUAL_VERSION;
        self.rebalance_executor = params.rebalance_executor;
    }
}

/// Initialize a depositor params
pub struct InitDepositorParams {
    /// Registry
    pub registry: Pubkey,
    /// Executor
    pub rebalance_executor: Pubkey,
}

impl Sealed for Depositor {}
impl Pack for Depositor {
    // 1 + 1 + 64
    const LEN: usize = 66;

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let mut slice = dst;
        self.serialize(&mut slice).unwrap()
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, solana_program::program_error::ProgramError> {
        Self::try_from_slice(src).map_err(|_| {
            msg!("Failed to deserialize");
            msg!("Actual LEN: {}", std::mem::size_of::<Depositor>());
            ProgramError::InvalidAccountData
        })
    }
}

impl IsInitialized for Depositor {
    fn is_initialized(&self) -> bool {
        self.account_type == AccountType::Depositor && self.account_version == Self::ACTUAL_VERSION
    }
}

impl Uninitialized for Depositor {
    fn is_uninitialized(&self) -> bool {
        self.account_type == AccountType::default()
    }
}
