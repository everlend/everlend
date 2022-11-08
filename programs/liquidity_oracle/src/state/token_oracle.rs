//! Token distribution state definitions.

use super::AccountType;
use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use everlend_registry::state::TOTAL_DISTRIBUTIONS;
use everlend_utils::{EverlendError, Uninitialized, PRECISION_SCALER};
use solana_program::{
    clock::Slot,
    msg,
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
};

pub type DistributionArray = [u64; TOTAL_DISTRIBUTIONS];

#[repr(C)]
#[derive(Debug, Clone, BorshDeserialize, BorshSerialize, BorshSchema, PartialEq, Default)]
pub struct Distribution {
    /// Current distribution array
    pub values: DistributionArray,
    /// Last update slot
    pub updated_at: Slot,
}

#[repr(C)]
#[derive(Debug, Clone, BorshDeserialize, BorshSerialize, BorshSchema, PartialEq, Default)]
pub struct TokenOracle {
    /// Account type.
    pub account_type: AccountType,

    /// MM liquidity distribution
    pub liquidity_distribution: Distribution,

    /// Liquidity to collateral rates of reserves
    pub reserve_rates: Distribution,
}

impl TokenOracle {
    /// Initialize a liquidity oracle.
    pub fn init() -> TokenOracle {
        TokenOracle {
            account_type: AccountType::TokenOracle,
            ..Default::default()
        }
    }

    /// Update a liquidity oracle token distribution
    pub fn update_liquidity_distribution(
        &mut self,
        slot: Slot,
        distribution: DistributionArray,
    ) -> Result<(), ProgramError> {
        // Total distribution always should be < 1 * PRECISION_SCALER
        let total_distribution = distribution
            .iter()
            .try_fold(0u64, |acc, &x| acc.checked_add(x))
            .ok_or(EverlendError::MathOverflow)?;
        if total_distribution > (PRECISION_SCALER) as u64 {
            return Err(ProgramError::InvalidArgument);
        }

        self.liquidity_distribution = Distribution {
            values: distribution,
            updated_at: slot,
        };

        Ok(())
    }

    /// Update a liquidity oracle token distribution
    pub fn update_reserve_rates(
        &mut self,
        slot: Slot,
        rates: DistributionArray,
    ) -> Result<(), ProgramError> {
        self.reserve_rates = Distribution {
            values: rates,
            updated_at: slot,
        };

        Ok(())
    }
}

impl Sealed for TokenOracle {}
impl Pack for TokenOracle {
    const LEN: usize = 1 + Distribution::LEN + Distribution::LEN;

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let mut slice = dst;
        self.serialize(&mut slice).unwrap()
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        Self::try_from_slice(src).map_err(|_| {
            msg!("Failed to deserialize");
            msg!("Actual LEN: {}", std::mem::size_of::<TokenOracle>());
            ProgramError::InvalidAccountData
        })
    }
}

impl IsInitialized for TokenOracle {
    fn is_initialized(&self) -> bool {
        self.account_type == AccountType::TokenOracle
    }
}

impl Uninitialized for TokenOracle {
    fn is_uninitialized(&self) -> bool {
        self.account_type == AccountType::default()
    }
}

impl Distribution {
    pub const LEN: usize = (8 * TOTAL_DISTRIBUTIONS) + 8;
}
