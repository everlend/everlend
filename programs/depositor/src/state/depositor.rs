//! Program state definitions

use super::AccountType;
use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::{
    msg,
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};

/// Depositor
#[repr(C)]
#[derive(Debug, BorshDeserialize, BorshSerialize, BorshSchema, Default)]
pub struct Depositor {
    /// Account type - Depositor
    pub account_type: AccountType,

    /// General pool market
    pub general_pool_market: Pubkey,

    /// Income pool market
    pub income_pool_market: Pubkey,

    /// Liquidity oracle
    pub liquidity_oracle: Pubkey,
}

impl Depositor {
    /// Initialize a voting pool
    pub fn init(&mut self, params: InitDepositorParams) {
        self.account_type = AccountType::Depositor;
        self.general_pool_market = params.general_pool_market;
        self.income_pool_market = params.income_pool_market;
        self.liquidity_oracle = params.liquidity_oracle;
    }
}

/// Initialize a depositor params
pub struct InitDepositorParams {
    /// General pool market
    pub general_pool_market: Pubkey,
    /// Income pool market
    pub income_pool_market: Pubkey,
    /// Liquidity oracle
    pub liquidity_oracle: Pubkey,
}

impl Sealed for Depositor {}
impl Pack for Depositor {
    // 1 + 32 + 32 + 32
    const LEN: usize = 97;

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
    }
}
