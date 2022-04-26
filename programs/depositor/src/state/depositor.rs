//! Program state definitions

use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::{
    msg,
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};

pub use deprecated::DeprecatedDepositor;
use everlend_utils::EverlendError;

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
    pub registry: Pubkey,
}

impl Depositor {
    /// Actual version of this struct
    pub const ACTUAL_VERSION: u8 = 1;

    /// Index of account type byte
    pub const ACCOUNT_TYPE_BYTE_INDEX: usize = 0;

    /// Reserved space for future values
    pub const FREE_SPACE: usize = 31;

    /// Create a depositor
    pub fn new(params: InitDepositorParams) -> Self {
        Self {
            account_type: AccountType::Depositor,
            version: Self::ACTUAL_VERSION,
            liquidity_oracle: params.liquidity_oracle,
            registry: params.registry,
        }
    }

    /// Initialize a depositor
    pub fn init(&mut self, params: InitDepositorParams) {
        self.account_type = AccountType::Depositor;
        self.version = Self::ACTUAL_VERSION;
        self.liquidity_oracle = params.liquidity_oracle;
        self.registry = params.registry;
    }
}

/// Initialize a depositor params
pub struct InitDepositorParams {
    /// Liquidity oracle
    pub liquidity_oracle: Pubkey,
    /// Registry
    pub registry: Pubkey,
}

impl Sealed for Depositor {}
impl Pack for Depositor {
    // 1 + 1 + 32 + 32 + 31
    const LEN: usize = 97;

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let mut slice = dst;
        self.serialize(&mut slice).unwrap()
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        if !src[Self::LEN - Self::FREE_SPACE..]
            .iter()
            .all(|byte| byte == &0)
        {
            Err(EverlendError::TemporaryUnavailable)?
        }

        Self::try_from_slice(&src[..Self::LEN - Self::FREE_SPACE]).map_err(|_| {
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
            if src[Depositor::LEN - Depositor::FREE_SPACE..]
                .iter()
                .all(|byte| byte == &0)
            {
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
