//! Liquidity oracle state definitions.

use super::AccountType;
use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use everlend_utils::Uninitialized;
use solana_program::{
    msg,
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};

/// Liquidity oracle initialization params.
pub struct InitLiquidityOracleParams {
    /// Authority.
    pub authority: Pubkey,
}

/// Liquidity oracle.
#[repr(C)]
#[derive(Debug, Clone, BorshDeserialize, BorshSerialize, BorshSchema, PartialEq, Default)]
pub struct LiquidityOracle {
    /// Account type.
    pub account_type: AccountType,
    /// Authority.
    pub authority: Pubkey,
}

impl LiquidityOracle {
    /// Initialize a liquidity oracle.
    pub fn init(&mut self, params: InitLiquidityOracleParams) {
        self.account_type = AccountType::LiquidityOracle;
        self.authority = params.authority;
    }

    /// Update liquidity oracle.
    pub fn update(&mut self, authority: Pubkey) {
        self.authority = authority;
    }
}

impl Sealed for LiquidityOracle {}

impl Pack for LiquidityOracle {
    // 1 + 32
    const LEN: usize = 33;

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let mut slice = dst;
        self.serialize(&mut slice).unwrap()
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        Self::try_from_slice(src).map_err(|_| {
            msg!("Failed to deserialize");
            msg!("Actual LEN: {}", std::mem::size_of::<LiquidityOracle>());
            ProgramError::InvalidAccountData
        })
    }
}

impl IsInitialized for LiquidityOracle {
    fn is_initialized(&self) -> bool {
        self.account_type == AccountType::LiquidityOracle
    }
}

impl Uninitialized for LiquidityOracle {
    fn is_uninitialized(&self) -> bool {
        self.account_type == AccountType::default()
    }
}
