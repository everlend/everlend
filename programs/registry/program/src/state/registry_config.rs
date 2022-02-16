//! Registry config state definitions

use super::*;
use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::{
    msg,
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};

/// Total number of money market distributions
pub const TOTAL_DISTRIBUTIONS: usize = 10;

/// Registry config
#[repr(C)]
#[derive(Debug, BorshDeserialize, BorshSerialize, BorshSchema, Default)]
pub struct RegistryConfig {
    /// Account type - RegistryConfig
    pub account_type: AccountType,
    /// Registry
    pub registry: Pubkey,
    /// ULP program
    pub ulp_program_id: Pubkey,
    /// Liquidity oracle program
    pub liquidity_oracle_program_id: Pubkey,
    /// Depositor program
    pub depositor_program_id: Pubkey,
    /// Income pools program
    pub income_pools_program_id: Pubkey,
    /// Money market programs
    pub money_market_program_ids: [Pubkey; TOTAL_DISTRIBUTIONS],
    // Space for future values
    // 543
}

impl RegistryConfig {
    /// Init a registry config
    pub fn init(&mut self, params: InitRegistryConfigParams) {
        self.account_type = AccountType::RegistryConfig;
        self.registry = params.registry;
    }

    /// Set a registry config
    pub fn set(&mut self, params: SetRegistryConfigParams) {
        self.ulp_program_id = params.ulp_program_id;
        self.liquidity_oracle_program_id = params.liquidity_oracle_program_id;
        self.depositor_program_id = params.depositor_program_id;
        self.income_pools_program_id = params.income_pools_program_id;
        self.money_market_program_ids = params.money_market_program_ids;
    }
}

/// Initialize a registry config params
pub struct InitRegistryConfigParams {
    /// Registry
    pub registry: Pubkey,
}

/// Set a registry config params
#[derive(Debug, BorshDeserialize, BorshSerialize, BorshSchema, PartialEq)]
pub struct SetRegistryConfigParams {
    /// ULP program
    pub ulp_program_id: Pubkey,
    /// Liquidity oracle program
    pub liquidity_oracle_program_id: Pubkey,
    /// Depositor program
    pub depositor_program_id: Pubkey,
    /// Income pools program
    pub income_pools_program_id: Pubkey,
    /// Money market programs
    pub money_market_program_ids: [Pubkey; TOTAL_DISTRIBUTIONS],
}

impl Sealed for RegistryConfig {}
impl Pack for RegistryConfig {
    // 1 + 32 + 32 + 32 + 32 + 32 + (10 * 32) + 543 = 1024
    const LEN: usize = 1024;

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

impl IsInitialized for RegistryConfig {
    fn is_initialized(&self) -> bool {
        self.account_type != AccountType::Uninitialized
            && self.account_type == AccountType::RegistryConfig
    }
}
