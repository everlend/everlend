//! Pool config state definitions

use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::{
    msg,
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
};

use super::*;
/// Pool config
#[repr(C)]
#[derive(Debug, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct PoolConfig {
    /// Account type - PoolConfig
    pub account_type: AccountType,
    /// Minimum amount for deposit
    pub deposit_minimum: u64,
    /// Minimum amount for withdraw request
    pub withdraw_minimum: u64,
}

impl PoolConfig {
    /// Init pool config
    pub fn default() -> PoolConfig {
        PoolConfig {
            account_type: AccountType::PoolConfig,
            deposit_minimum: 0,
            withdraw_minimum: 0,
        }
    }

    /// Set pool config
    pub fn set(&mut self, params: SetPoolConfigParams) {
        if params.deposit_minimum.is_some() {
            self.deposit_minimum = params.deposit_minimum.unwrap();
        }

        if params.withdraw_minimum.is_some() {
            self.withdraw_minimum = params.withdraw_minimum.unwrap();
        }
    }
}

impl Sealed for PoolConfig {}
impl Pack for PoolConfig {
    const LEN: usize = 1 + 8 + 8;

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

impl IsInitialized for PoolConfig {
    fn is_initialized(&self) -> bool {
        self.account_type == AccountType::PoolConfig
    }
}

/// Set pool config params
#[derive(Debug, BorshDeserialize, BorshSerialize, BorshSchema, PartialEq, Clone, Copy)]
pub struct SetPoolConfigParams {
    /// Minimum amount for deposit
    pub deposit_minimum: Option<u64>,
    /// Minimum amount for withdraw request
    pub withdraw_minimum: Option<u64>,
}
