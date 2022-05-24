//! Pool config state definitions

use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::{
    msg,
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};

use super::*;
/// Pool config
#[repr(C)]
#[derive(Debug, BorshDeserialize, BorshSerialize, BorshSchema, Default)]
pub struct RegistryPoolConfig {
    /// Account type - RegistryPoolConfig
    pub account_type: AccountType,
    /// Registry
    pub registry: Pubkey,
    /// General pool program
    pub general_pool: Pubkey,
    /// Minimum amount for deposit
    pub deposit_minimum: u64,
    /// Minimum amount for withdraw request
    pub withdraw_minimum: u64,
}

impl RegistryPoolConfig {
    /// Init pool config
    pub fn init(&mut self, registry: Pubkey, general_pool: Pubkey) {
        self.registry = registry;
        self.general_pool = general_pool;
        self.account_type = AccountType::RegistryPoolConfig;
    }

    /// Set pool config
    pub fn set(&mut self, params: SetRegistryPoolConfigParams) {
        self.deposit_minimum = params.deposit_minimum;
        self.withdraw_minimum = params.withdraw_minimum;
    }
}


impl Sealed for RegistryPoolConfig {}
impl Pack for RegistryPoolConfig {
    // 1 + 32 + 32 + 8 + 8 = 81
    const LEN: usize = 81;

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let mut slice = dst;
        self.serialize(&mut slice).unwrap()
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let mut src_mut = src;
        Self::deserialize(&mut src_mut).map_err(|err| {
            msg!("Failed to deserialize");
            msg!(&err.to_string());
            ProgramError::InvalidAccountData
        })
    }
}

impl IsInitialized for RegistryPoolConfig {
    fn is_initialized(&self) -> bool {
        self.account_type == AccountType::RegistryPoolConfig
    }
}

/// Set pool config params
#[derive(Debug, BorshDeserialize, BorshSerialize, BorshSchema, PartialEq, Clone, Copy)]
pub struct SetRegistryPoolConfigParams {
    /// Minimum amount for deposit
    pub deposit_minimum: u64,
    /// Minimum amount for withdraw request
    pub withdraw_minimum: u64,
}
