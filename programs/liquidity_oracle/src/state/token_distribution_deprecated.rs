//! Token distribution state definitions.

use super::{AccountType, DistributionArray};
use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use everlend_registry::state::TOTAL_DISTRIBUTIONS;
use everlend_utils::Uninitialized;
use solana_program::{
    clock::Slot,
    msg,
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
};

#[repr(C)]
#[derive(Debug, Clone, BorshDeserialize, BorshSerialize, BorshSchema, PartialEq, Default)]
pub struct DeprecatedTokenDistribution {
    // Account type.
    pub account_type: AccountType,

    // Current distribution array
    pub distribution: DistributionArray,

    // Last update slot
    pub updated_at: Slot,
}

impl Sealed for DeprecatedTokenDistribution {}
impl Pack for DeprecatedTokenDistribution {
    // 1 + (8 * 10) + 8 = 89
    const LEN: usize = 1 + (8 * TOTAL_DISTRIBUTIONS) + 8;

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let mut slice = dst;
        self.serialize(&mut slice).unwrap()
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        Self::try_from_slice(src).map_err(|_| {
            msg!("Failed to deserialize");
            msg!(
                "Actual LEN: {}",
                std::mem::size_of::<DeprecatedTokenDistribution>()
            );
            ProgramError::InvalidAccountData
        })
    }
}

impl IsInitialized for DeprecatedTokenDistribution {
    fn is_initialized(&self) -> bool {
        self.account_type == AccountType::TokenOracle
    }
}

impl Uninitialized for DeprecatedTokenDistribution {
    fn is_uninitialized(&self) -> bool {
        self.account_type == AccountType::default()
    }
}
