//! Program state definitions

use super::AccountType;
use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use everlend_utils::Uninitialized;
use solana_program::{
    msg,
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};

/// Registry
#[repr(C)]
#[derive(Debug, BorshDeserialize, BorshSerialize, BorshSchema, Default)]
pub struct Registry {
    /// Account type - Registry
    pub account_type: AccountType,

    /// Manager
    pub manager: Pubkey,
}

impl Registry {
    /// Initialize a voting pool
    pub fn init(&mut self, params: InitRegistryParams) {
        self.account_type = AccountType::Registry;
        self.manager = params.manager;
    }
}

/// Initialize a registry params
pub struct InitRegistryParams {
    /// Manager
    pub manager: Pubkey,
}

impl Sealed for Registry {}
impl Pack for Registry {
    // 1 + 32
    const LEN: usize = 33;

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let mut slice = dst;
        self.serialize(&mut slice).unwrap()
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        Self::try_from_slice(src).map_err(|_| {
            msg!("Failed to deserialize");
            ProgramError::InvalidAccountData
        })
    }
}

impl IsInitialized for Registry {
    fn is_initialized(&self) -> bool {
        self.account_type == AccountType::Registry
    }
}

impl Uninitialized for Registry {
    fn is_uninitialized(&self) -> bool {
        self.account_type == AccountType::default()
    }
}
