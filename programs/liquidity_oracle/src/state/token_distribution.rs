//! Token distribution state definitions.

use super::AccountType;
use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use everlend_registry::state::TOTAL_DISTRIBUTIONS;
use everlend_utils::{Uninitialized, PRECISION_SCALER};
use solana_program::{
    clock::Slot,
    msg,
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
};

pub type DistributionArray = [u64; TOTAL_DISTRIBUTIONS];

#[repr(C)]
#[derive(Debug, Clone, BorshDeserialize, BorshSerialize, BorshSchema, PartialEq, Default)]
pub struct TokenDistribution {
    /// Account type.
    pub account_type: AccountType,

    /// Current distribution array
    pub distribution: DistributionArray,

    /// Collateral reserve rates
    pub reserve_rates: DistributionArray,

    /// Last update slot for distrubution
    pub updated_at: Slot,

    /// Last update slot for reserve rates
    pub reserve_rates_updated_at: Slot,
}

impl TokenDistribution {
    /// Initialize a liquidity oracle.
    pub fn init() -> TokenDistribution {
        TokenDistribution {
            account_type: AccountType::TokenDistribution,
            ..Default::default()
        }
    }

    /// Update a liquidity oracle token distribution
    pub fn update_distribution(
        &mut self,
        slot: Slot,
        distribution: DistributionArray,
    ) -> Result<(), ProgramError> {
        // Total distribution always should be < 1 * PRECISION_SCALER
        if distribution.iter().sum::<u64>() > (PRECISION_SCALER) as u64 {
            return Err(ProgramError::InvalidArgument);
        }

        self.distribution = distribution;
        self.updated_at = slot;

        Ok(())
    }
}

impl Sealed for TokenDistribution {}
impl Pack for TokenDistribution {
    const LEN: usize = 1 + (8 * TOTAL_DISTRIBUTIONS) + (8 * TOTAL_DISTRIBUTIONS) + 8 + 8;

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let mut slice = dst;
        self.serialize(&mut slice).unwrap()
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        Self::try_from_slice(src).map_err(|_| {
            msg!("Failed to deserialize");
            msg!("Actual LEN: {}", std::mem::size_of::<TokenDistribution>());
            ProgramError::InvalidAccountData
        })
    }
}

impl IsInitialized for TokenDistribution {
    fn is_initialized(&self) -> bool {
        self.account_type == AccountType::TokenDistribution
    }
}

impl Uninitialized for TokenDistribution {
    fn is_uninitialized(&self) -> bool {
        self.account_type == AccountType::default()
    }
}
