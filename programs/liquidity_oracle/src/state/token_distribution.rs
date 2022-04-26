//! Token distribution state definitions.

use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::{
    clock::Slot,
    msg,
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
};

pub use deprecated::DeprecatedTokenDistribution;
use everlend_registry::state::TOTAL_DISTRIBUTIONS;
use everlend_utils::EverlendError;

use super::AccountType;

pub type DistributionArray = [u64; TOTAL_DISTRIBUTIONS];

#[repr(C)]
#[derive(Debug, Clone, BorshDeserialize, BorshSerialize, BorshSchema, PartialEq, Default)]
pub struct TokenDistribution {
    /// Account type
    pub account_type: AccountType,
    /// Struct version
    pub version: u8,
    /// Current distribution array
    pub distribution: DistributionArray,
    /// Last update slot
    pub updated_at: Slot,
    // Space for future values
    // 39
}

impl TokenDistribution {
    /// Actual version of this struct
    pub const ACTUAL_VERSION: u8 = 1;

    /// Reserved space for future values
    pub const FREE_SPACE: usize = 39;

    /// Initialize a liquidity oracle.
    pub fn init(&mut self) {
        self.account_type = AccountType::TokenDistribution;
        self.version = Self::ACTUAL_VERSION;
    }

    /// Update a liquidity oracle token distribution
    pub fn update(&mut self, slot: Slot, distribution: DistributionArray) {
        self.updated_at = slot;
        self.distribution = distribution;
    }
}

impl Sealed for TokenDistribution {}
impl Pack for TokenDistribution {
    // 1 + 1 + (8 * 5) + 8 + 39 = 89
    const LEN: usize = 89;

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

impl IsInitialized for TokenDistribution {
    fn is_initialized(&self) -> bool {
        self.account_type != AccountType::Uninitialized
            && self.account_type == AccountType::TokenDistribution
            && self.version == Self::ACTUAL_VERSION
    }
}

mod deprecated {
    use std::convert::TryInto;

    use super::*;

    #[repr(C)]
    #[derive(Debug, Clone, BorshDeserialize, BorshSerialize, BorshSchema, PartialEq, Default)]
    pub struct DeprecatedTokenDistribution {
        // Account type.
        pub account_type: AccountType,
        // Current distribution array
        pub distribution: [u64; 10],
        // Last update slot
        pub updated_at: Slot,
    }

    impl From<DeprecatedTokenDistribution> for TokenDistribution {
        fn from(deprecated: DeprecatedTokenDistribution) -> Self {
            Self {
                account_type: AccountType::TokenDistribution,
                version: Self::ACTUAL_VERSION,
                distribution: deprecated.distribution[..TOTAL_DISTRIBUTIONS]
                    .try_into()
                    .unwrap(),
                updated_at: deprecated.updated_at,
            }
        }
    }

    impl Sealed for DeprecatedTokenDistribution {}

    impl Pack for DeprecatedTokenDistribution {
        const LEN: usize = 89;

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

    impl IsInitialized for DeprecatedTokenDistribution {
        fn is_initialized(&self) -> bool {
            self.account_type != AccountType::Uninitialized
                && self.account_type == AccountType::TokenDistribution
        }
    }
}
