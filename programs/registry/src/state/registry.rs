//! Program state definitions

use super::AccountType;
use crate::state::{DeprecatedRegistry, DeprecatedRegistryMarkets};
use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use everlend_utils::integrations::MoneyMarket;
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

/// Money Markets
pub type MoneyMarkets = [MoneyMarket; TOTAL_DISTRIBUTIONS];

/// Distribution pubkeys
pub type DistributionPubkeys = [Pubkey; TOTAL_DISTRIBUTIONS];

const REGISTRY_LEN: usize = 1 + (32 + 32 + 32 + 32 + 8);
const REGISTRY_MONEY_MARKET_LEN: usize = 1 + 32;
const REGISTRY_MARKETS_LEN: usize =
    (REGISTRY_MONEY_MARKET_LEN * TOTAL_DISTRIBUTIONS) + (32 * TOTAL_DISTRIBUTIONS);

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
    // pub money_market_program_ids: MoneyMarkets,
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

    /// Migrate registry
    pub fn migrate(deprecated_registry: &DeprecatedRegistry) -> Registry {
        Registry {
            account_type: deprecated_registry.account_type.clone(),
            manager: deprecated_registry.manager,
            general_pool_market: deprecated_registry.general_pool_market,
            income_pool_market: deprecated_registry.income_pool_market,
            liquidity_oracle: deprecated_registry.liquidity_oracle,
            refresh_income_interval: deprecated_registry.refresh_income_interval,
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
    /// Money markets
    pub money_markets: MoneyMarkets,
    /// Collateral pool market program ids
    pub collateral_pool_markets: DistributionPubkeys,
}

impl RegistryMarkets {
    /// Migrate registry markets
    pub fn migrate(
        deprecated_markets: &DeprecatedRegistryMarkets,
        money_markets: MoneyMarkets,
    ) -> RegistryMarkets {
        RegistryMarkets {
            money_markets,
            collateral_pool_markets: deprecated_markets.collateral_pool_markets,
        }
    }

    /// Unpack collateral pool from slice
    pub fn unpack_collateral_pool_markets(src: &[u8]) -> Result<DistributionPubkeys, ProgramError> {
        let mut src_mut = &src[REGISTRY_LEN + (REGISTRY_MONEY_MARKET_LEN * TOTAL_DISTRIBUTIONS)
            ..REGISTRY_LEN + REGISTRY_MARKETS_LEN];

        DistributionPubkeys::deserialize(&mut src_mut).map_err(|err| {
            msg!("Failed to deserialize");
            msg!(&err.to_string());
            ProgramError::InvalidAccountData
        })
    }

    ///
    pub fn unpack_money_markets(src: &[u8]) -> Result<MoneyMarkets, ProgramError> {
        let mut src_mut =
            &src[REGISTRY_LEN..REGISTRY_LEN + REGISTRY_MONEY_MARKET_LEN * TOTAL_DISTRIBUTIONS];

        MoneyMarkets::deserialize(&mut src_mut).map_err(|err| {
            msg!("Failed to deserialize");
            msg!(&err.to_string());
            ProgramError::InvalidAccountData
        })
    }

    ///
    pub fn unpack_money_markets_with_index(
        src: &[u8],
        index: usize,
    ) -> Result<MoneyMarket, ProgramError> {
        if index >= TOTAL_DISTRIBUTIONS {
            return Err(ProgramError::InvalidArgument);
        }

        let mut src_mut = &src[REGISTRY_LEN + (REGISTRY_MONEY_MARKET_LEN * index)
            ..REGISTRY_LEN + (REGISTRY_MONEY_MARKET_LEN * (index + 1))];
        MoneyMarket::deserialize(&mut src_mut).map_err(|err| {
            msg!("Failed to deserialize");
            msg!(&err.to_string());
            ProgramError::InvalidAccountData
        })
    }
}

impl Sealed for RegistryMarkets {}
impl Pack for RegistryMarkets {
    const LEN: usize = REGISTRY_MARKETS_LEN;

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let mut slice = Vec::with_capacity(dst.len());
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
