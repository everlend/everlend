//! Program state definitions

use super::AccountType;
use crate::state::{DistributionPubkeys, TOTAL_DISTRIBUTIONS};
use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use everlend_utils::Uninitialized;
use solana_program::{
    clock::Slot,
    msg,
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};

const REGISTRY_LEN: usize = 1 + (32 + 32 + 32 + 32 + 8);
const REGISTRY_MARKETS_LEN: usize = (32 * TOTAL_DISTRIBUTIONS) + (32 * TOTAL_DISTRIBUTIONS);

/// Registry
#[repr(C)]
#[derive(Debug, BorshDeserialize, BorshSerialize, BorshSchema, Default)]
pub struct DeprecatedRegistry {
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

impl DeprecatedRegistry {
    /// Initialize a voting pool
    pub fn init(manager: Pubkey) -> DeprecatedRegistry {
        DeprecatedRegistry {
            account_type: AccountType::Registry,
            manager,
            ..Default::default()
        }
    }
}

impl Sealed for DeprecatedRegistry {}
impl Pack for DeprecatedRegistry {
    const LEN: usize = REGISTRY_LEN + REGISTRY_MARKETS_LEN;

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let mut slice = Vec::with_capacity(REGISTRY_LEN);
        self.serialize(&mut slice).unwrap();
        dst[0..REGISTRY_LEN].copy_from_slice(&slice)
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let mut src_mut = &src[0..REGISTRY_LEN];
        Self::deserialize(&mut src_mut).map_err(|err| {
            msg!("Failed to deserialize");
            msg!(&err.to_string());
            ProgramError::InvalidAccountData
        })
    }
}

impl IsInitialized for DeprecatedRegistry {
    fn is_initialized(&self) -> bool {
        self.account_type == AccountType::Registry && !self.manager.eq(&Pubkey::default())
    }
}

impl Uninitialized for DeprecatedRegistry {
    fn is_uninitialized(&self) -> bool {
        self.account_type == AccountType::default()
    }
}

/// Registry programs
#[repr(C)]
#[derive(Debug, BorshDeserialize, BorshSerialize, BorshSchema, Default, PartialEq, Copy, Clone)]
pub struct DeprecatedRegistryMarkets {
    /// Money market program ids
    pub money_markets: DistributionPubkeys,
    /// Collateral pool market program ids
    pub collateral_pool_markets: DistributionPubkeys,
}

impl Sealed for DeprecatedRegistryMarkets {}
impl Pack for DeprecatedRegistryMarkets {
    const LEN: usize = REGISTRY_MARKETS_LEN;

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let mut slice = Vec::with_capacity(REGISTRY_MARKETS_LEN);
        self.serialize(&mut slice).unwrap();

        dst[REGISTRY_LEN..REGISTRY_LEN + REGISTRY_MARKETS_LEN].copy_from_slice(&slice)
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let mut src_mut = &src[REGISTRY_LEN..REGISTRY_LEN + REGISTRY_MARKETS_LEN];

        Self::deserialize(&mut src_mut).map_err(|err| {
            msg!("Failed to deserialize");
            msg!(&err.to_string());
            ProgramError::InvalidAccountData
        })
    }
}