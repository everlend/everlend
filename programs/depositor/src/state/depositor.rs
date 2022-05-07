//! Program state definitions

use super::AccountType;
use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::{
    msg,
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};

// 1 + 32
const DEPOSITOR_LEN: usize = 33;

/// Depositor
#[repr(C)]
#[derive(Debug, BorshDeserialize, BorshSerialize, BorshSchema, Default)]
pub struct Depositor {
    /// Account type - Depositor
    pub account_type: AccountType,

    /// Registry
    pub registry: Pubkey,
}

impl Depositor {
    /// Initialize a voting pool
    pub fn init(&mut self, params: InitDepositorParams) {
        self.account_type = AccountType::Depositor;
        self.registry = params.registry;
    }
}

/// Initialize a depositor params
pub struct InitDepositorParams {
    /// Registry
    pub registry: Pubkey,
}

impl Sealed for Depositor {}
impl Pack for Depositor {
    // 1 + 32 + 64
    const LEN: usize = 97;

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let mut slice = Vec::with_capacity(DEPOSITOR_LEN);
        self.serialize(&mut slice).unwrap();
        dst[0..DEPOSITOR_LEN].copy_from_slice(&slice)
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        Self::try_from_slice(&src[0..DEPOSITOR_LEN]).map_err(|_| {
            msg!("Failed to deserialize");
            ProgramError::InvalidAccountData
        })
    }
}

impl IsInitialized for Depositor {
    fn is_initialized(&self) -> bool {
        self.account_type != AccountType::Uninitialized
            && self.account_type == AccountType::Depositor
    }
}
