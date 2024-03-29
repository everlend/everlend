//! Program state definitions

use super::AccountType;
use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use everlend_utils::Uninitialized;
use solana_program::{
    clock::Slot,
    msg,
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};

/// Total number of money market distributions
pub const TOTAL_DISTRIBUTIONS: usize = 10;

/// Distribution pubkeys
pub type DistributionPubkeys = [Pubkey; TOTAL_DISTRIBUTIONS];

const REGISTRY_LEN: usize = 1 + (32 + 32 + 32 + 32 + 8);
const REGISTRY_MARKETS_LEN: usize = (32 * TOTAL_DISTRIBUTIONS) + (32 * TOTAL_DISTRIBUTIONS);

/// Registry
#[repr(C)]
#[derive(Debug, BorshDeserialize, BorshSerialize, BorshSchema, Default)]
pub struct Registry {
    /// Account type - Registry
    pub account_type: AccountType,
    /// The address responsible for core settings and instructions.
    pub manager: Pubkey,
    /// General pool market
    pub general_pool_market: Pubkey,
    /// Income pool market
    pub income_pool_market: Pubkey,
    /// Liquidity oracle
    pub liquidity_oracle: Pubkey,
    /// Refresh income interval
    pub refresh_income_interval: Slot,
    // Program ids for money markets
    // pub money_market_program_ids: DistributionPubkeys,
    // Collateral pool markets
    // pub collateral_pool_markets: DistributionPubkeys,
}

impl Registry {
    /// Initialize a voting pool
    pub fn init(manager: Pubkey) -> Registry {
        Registry {
            account_type: AccountType::Registry,
            manager,
            ..Default::default()
        }
    }
}

impl Sealed for Registry {}
impl Pack for Registry {
    const LEN: usize = REGISTRY_LEN + REGISTRY_MARKETS_LEN;

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let mut slice = Vec::with_capacity(REGISTRY_LEN);
        self.serialize(&mut slice).unwrap();
        dst[0..REGISTRY_LEN].copy_from_slice(&slice)
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        if src.len() != REGISTRY_LEN + REGISTRY_MARKETS_LEN {
            return Err(ProgramError::InvalidAccountData);
        }

        let mut src_mut = &src[0..REGISTRY_LEN];
        Self::deserialize(&mut src_mut).map_err(|err| {
            msg!("Failed to deserialize");
            msg!(&err.to_string());
            ProgramError::InvalidAccountData
        })
    }
}

impl IsInitialized for Registry {
    fn is_initialized(&self) -> bool {
        self.account_type == AccountType::Registry && !self.manager.eq(&Pubkey::default())
    }
}

impl Uninitialized for Registry {
    fn is_uninitialized(&self) -> bool {
        self.account_type == AccountType::default()
    }
}

/// Registry programs
#[repr(C)]
#[derive(Debug, BorshDeserialize, BorshSerialize, BorshSchema, Default, PartialEq, Copy, Clone)]
pub struct RegistryMarkets {
    /// Money market program ids
    pub money_markets: DistributionPubkeys,
    /// Collateral pool market program ids
    pub collateral_pool_markets: DistributionPubkeys,
}

impl Sealed for RegistryMarkets {}
impl Pack for RegistryMarkets {
    const LEN: usize = REGISTRY_MARKETS_LEN;

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let mut slice = Vec::with_capacity(REGISTRY_MARKETS_LEN);
        self.serialize(&mut slice).unwrap();

        dst[REGISTRY_LEN..REGISTRY_LEN + REGISTRY_MARKETS_LEN].copy_from_slice(&slice)
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        if src.len() != REGISTRY_LEN + REGISTRY_MARKETS_LEN {
            return Err(ProgramError::InvalidAccountData);
        }

        let mut src_mut = &src[REGISTRY_LEN..REGISTRY_LEN + REGISTRY_MARKETS_LEN];

        Self::deserialize(&mut src_mut).map_err(|err| {
            msg!("Failed to deserialize");
            msg!(&err.to_string());
            ProgramError::InvalidAccountData
        })
    }
}

#[cfg(test)]
pub mod tests {
    use crate::state::registry::{REGISTRY_LEN, REGISTRY_MARKETS_LEN};
    use crate::state::{Registry, RegistryMarkets};
    use solana_program::program_error::ProgramError;
    use solana_program::program_pack::Pack;

    #[test]
    fn unpack_registry() {
        // Valid data size case
        let data = vec![0u8; REGISTRY_LEN + REGISTRY_MARKETS_LEN];

        Registry::unpack_from_slice(&data).unwrap();
        RegistryMarkets::unpack_from_slice(&data).unwrap();

        // Invalid data size case
        let wrong_sized_data = vec![0u8; REGISTRY_LEN + REGISTRY_MARKETS_LEN + 1];

        assert_eq!(
            Registry::unpack_from_slice(&wrong_sized_data).unwrap_err(),
            ProgramError::InvalidAccountData,
        );
        assert_eq!(
            RegistryMarkets::unpack_from_slice(&wrong_sized_data).unwrap_err(),
            ProgramError::InvalidAccountData,
        )
    }
}
