//! Program state definitions

use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
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
    /// Struct version
    pub version: u8,
    /// Liquidity oracle
    pub liquidity_oracle: Pubkey,
    /// Registry
    pub registry_config: Pubkey,
}

impl Depositor {
    /// Actual version of this struct
    pub const ACTUAL_VERSION: u8 = 1;

    /// Index of account type byte
    pub const ACCOUNT_TYPE_BYTE_INDEX: usize = 0;

    /// Create a voting pool
    pub fn new(params: InitDepositorParams) -> Self {
        Self {
            account_type: AccountType::Depositor,
            version: Self::ACTUAL_VERSION,
            liquidity_oracle: params.liquidity_oracle,
            registry_config: params.registry_config,
        }
    }
}

/// Initialize a depositor params
pub struct InitDepositorParams {
    /// Liquidity oracle
    pub liquidity_oracle: Pubkey,
    /// Registry
    pub registry_config: Pubkey,
}

impl Sealed for Depositor {}
impl Pack for Depositor {
    // 1 + 1 + 32 + 32
    const LEN: usize = 66;

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

impl IsInitialized for Depositor {
    fn is_initialized(&self) -> bool {
        self.account_type != AccountType::Uninitialized
            && self.account_type == AccountType::Depositor
            && self.version == Self::ACTUAL_VERSION
    }
}
