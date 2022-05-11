//! Registry config state definitions

use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::{
    clock::Slot,
    msg,
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};

pub use deprecated::DeprecatedRegistryConfig;
use everlend_utils::AccountVersion;

use super::*;

/// Total number of money market distributions
pub const TOTAL_DISTRIBUTIONS: usize = 10;

/// Distribution pubkeys
pub type DistributionPubkeys = [Pubkey; TOTAL_DISTRIBUTIONS];

const CONFIG_LEN: usize = 34;
const PROGRAMS_OFFSET: usize = CONFIG_LEN;
const PROGRAMS_LEN: usize = 480;
const ROOTS_OFFSET: usize = PROGRAMS_OFFSET + PROGRAMS_LEN;
const ROOTS_LEN: usize = 416;
const SETTINGS_OFFSET: usize = ROOTS_OFFSET + ROOTS_LEN;
const SETTINGS_LEN: usize = 8;

/// Registry programs
#[repr(C)]
#[derive(Debug, BorshDeserialize, BorshSerialize, BorshSchema, Default, PartialEq, Copy, Clone)]
pub struct RegistryPrograms {
    /// General pool program
    pub general_pool_program_id: Pubkey,
    /// ULP program
    pub ulp_program_id: Pubkey,
    /// Liquidity oracle program
    pub liquidity_oracle_program_id: Pubkey,
    /// Depositor program
    pub depositor_program_id: Pubkey,
    /// Income pools program
    pub income_pools_program_id: Pubkey,
    /// Money market programs
    pub money_market_program_ids: DistributionPubkeys,
}

impl Sealed for RegistryPrograms {}
impl Pack for RegistryPrograms {
    // 32 + 32 + 32 + 32 + 32 + (10 * 32) = 480
    const LEN: usize = PROGRAMS_LEN;

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let mut slice = Vec::with_capacity(PROGRAMS_LEN);
        self.serialize(&mut slice).unwrap();
        dst[PROGRAMS_OFFSET..PROGRAMS_OFFSET + PROGRAMS_LEN].copy_from_slice(&slice)
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let mut src_mut = &src[PROGRAMS_OFFSET..PROGRAMS_OFFSET + PROGRAMS_LEN];
        Self::deserialize(&mut src_mut).map_err(|err| {
            msg!("Failed to deserialize");
            msg!(&err.to_string());
            ProgramError::InvalidAccountData
        })
    }
}

/// Registry root accounts
#[derive(Debug, BorshDeserialize, BorshSerialize, BorshSchema, PartialEq, Default, Clone, Copy)]
pub struct RegistryRootAccounts {
    /// General pool market
    pub general_pool_market: Pubkey,
    /// Income pool market
    pub income_pool_market: Pubkey,
    /// Collateral pool markets
    pub collateral_pool_markets: DistributionPubkeys,
    /// Liquidity oracle
    pub liquidity_oracle: Pubkey,
}

impl RegistryRootAccounts {
    /// Return filtered from zero pubkeys iterator over ulp_pool_markets
    pub fn iter_filtered_ulp_pool_markets(&self) -> impl Iterator<Item = &Pubkey> {
        self.collateral_pool_markets
            .iter()
            .filter(|collateral_pool_markets| collateral_pool_markets != &&Pubkey::default())
    }
}

impl Sealed for RegistryRootAccounts {}

impl Pack for RegistryRootAccounts {
    const LEN: usize = ROOTS_LEN;

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let mut slice = Vec::with_capacity(ROOTS_LEN);
        self.serialize(&mut slice).unwrap();
        dst[ROOTS_OFFSET..ROOTS_OFFSET + ROOTS_LEN].copy_from_slice(&slice)
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let mut src_mut = &src[ROOTS_OFFSET..ROOTS_OFFSET + ROOTS_LEN];
        Self::deserialize(&mut src_mut).map_err(|err| {
            msg!("Failed to deserialize");
            msg!(&err.to_string());
            ProgramError::InvalidAccountData
        })
    }
}

/// Registry settings
#[derive(Debug, BorshDeserialize, BorshSerialize, BorshSchema, PartialEq, Default, Clone, Copy)]
pub struct RegistrySettings {
    /// Refresh income interval
    pub refresh_income_interval: Slot,
}

impl Sealed for RegistrySettings {}
impl Pack for RegistrySettings {
    // 8
    const LEN: usize = SETTINGS_LEN;

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let mut slice = Vec::with_capacity(SETTINGS_LEN);
        self.serialize(&mut slice).unwrap();
        dst[SETTINGS_OFFSET..SETTINGS_OFFSET + SETTINGS_LEN].copy_from_slice(&slice)
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let mut src_mut = &src[SETTINGS_OFFSET..SETTINGS_OFFSET + SETTINGS_LEN];
        Self::deserialize(&mut src_mut).map_err(|err| {
            msg!("Failed to deserialize");
            msg!(&err.to_string());
            ProgramError::InvalidAccountData
        })
    }
}

/// Registry config
#[repr(C)]
#[derive(Debug, BorshDeserialize, BorshSerialize, BorshSchema, Default)]
pub struct RegistryConfig {
    /// Account type - RegistryConfig
    pub account_type: AccountType,

    /// Account version
    pub account_version: AccountVersion,

    /// Registry
    pub registry: Pubkey,
    // ...
    // pub programs: RegistryPrograms,
    // pub roots: RegistryRootAccounts,
    // pub settings: RegistrySettings,
}

impl RegistryConfig {
    /// Account actual version
    const ACTUAL_VERSION: AccountVersion = AccountVersion::V0;

    /// Init a registry config
    pub fn init(&mut self, params: InitRegistryConfigParams) {
        self.account_type = AccountType::RegistryConfig;
        self.account_version = Self::ACTUAL_VERSION;
        self.registry = params.registry;
    }
}

/// Initialize a registry config params
pub struct InitRegistryConfigParams {
    /// Registry
    pub registry: Pubkey,
}

impl Sealed for RegistryConfig {}
impl Pack for RegistryConfig {
    const LEN: usize = 4096;

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let mut slice = Vec::with_capacity(CONFIG_LEN);
        self.serialize(&mut slice).unwrap();
        dst[0..CONFIG_LEN].copy_from_slice(&slice)
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let mut src_mut = &src[0..CONFIG_LEN];
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
            && self.account_version == Self::ACTUAL_VERSION
    }
}

mod deprecated {
    use super::*;

    ///
    #[repr(C)]
    #[derive(Debug, BorshDeserialize, BorshSerialize, BorshSchema, Default)]
    pub struct DeprecatedRegistryConfig {
        /// Account type - RegistryConfig
        pub account_type: AccountType,
        /// Registry
        pub registry: Pubkey,
        /// General pool program
        pub general_pool_program_id: Pubkey,
        /// ULP program
        pub ulp_program_id: Pubkey,
        /// Liquidity oracle program
        pub liquidity_oracle_program_id: Pubkey,
        /// Depositor program
        pub depositor_program_id: Pubkey,
        /// Income pools program
        pub income_pools_program_id: Pubkey,
        /// Money market programs
        pub money_market_program_ids: [Pubkey; 10],
        /// Refresh income interval
        pub refresh_income_interval: Slot,
    }

    impl Sealed for DeprecatedRegistryConfig {}

    impl Pack for DeprecatedRegistryConfig {
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

    impl IsInitialized for DeprecatedRegistryConfig {
        fn is_initialized(&self) -> bool {
            self.account_type != AccountType::Uninitialized
                && self.account_type == AccountType::RegistryConfig
        }
    }
}
