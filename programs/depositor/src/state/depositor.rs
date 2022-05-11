//! Program state definitions

use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::{
    msg,
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};

pub use deprecated::DeprecatedDepositor;
use everlend_utils::{AccountVersion, EverlendError};

use super::AccountType;

// 1 + 1 + 32
const DEPOSITOR_LEN: usize = 34;

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
}

impl Depositor {
    /// Account actual version
    const ACTUAL_VERSION: AccountVersion = AccountVersion::V0;

    /// Initialize a voting pool
    pub fn init(&mut self, params: InitDepositorParams) {
        self.account_type = AccountType::Depositor;
        self.registry = params.registry;
        self.account_version = Self::ACTUAL_VERSION;
    }
}

/// Initialize a depositor params
pub struct InitDepositorParams {
    /// Registry
    pub registry: Pubkey,
}

impl Sealed for Depositor {}
impl Pack for Depositor {
    // 1 + 1 + 32 + 63
    const LEN: usize = 97;

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let mut slice = Vec::with_capacity(DEPOSITOR_LEN);
        self.serialize(&mut slice).unwrap();
        dst[0..DEPOSITOR_LEN].copy_from_slice(&slice)
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        if !src[DEPOSITOR_LEN..].iter().all(|byte| byte == &0) {
            Err(EverlendError::TemporaryUnavailable)?
        }
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
            && self.account_version == Self::ACTUAL_VERSION
    }
}

mod deprecated {
    use super::*;

    ///
    #[repr(C)]
    #[derive(Debug, BorshDeserialize, BorshSerialize, BorshSchema, Default)]
    pub struct DeprecatedDepositor {
        /// Account type - Depositor
        pub account_type: AccountType,

        /// General pool market
        pub general_pool_market: Pubkey,

        /// Income pool market
        pub income_pool_market: Pubkey,

        /// Liquidity oracle
        pub liquidity_oracle: Pubkey,
    }

    impl Sealed for DeprecatedDepositor {}

    impl Pack for DeprecatedDepositor {
        // 1 + 32 + 32 + 32
        const LEN: usize = 97;

        fn pack_into_slice(&self, dst: &mut [u8]) {
            let mut slice = dst;
            self.serialize(&mut slice).unwrap()
        }

        fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
            if src[DEPOSITOR_LEN..].iter().all(|byte| byte == &0) {
                Err(EverlendError::TemporaryUnavailable)?
            }
            Self::try_from_slice(src).map_err(|_| {
                msg!("Failed to deserialize");
                ProgramError::InvalidAccountData
            })
        }
    }

    impl IsInitialized for DeprecatedDepositor {
        fn is_initialized(&self) -> bool {
            self.account_type != AccountType::Uninitialized
                && self.account_type == AccountType::Depositor
        }
    }
}
