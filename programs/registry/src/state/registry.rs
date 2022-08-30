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
    /// Manager that can upgrade Liquidity Oracle
    pub liquidity_oracle_manager: Pubkey,
    /// Program ids for money markets
    pub money_market_program_ids: DistributionPubkeys,
    /// Collateral pool markets
    pub collateral_pool_markets: DistributionPubkeys,
    /// Refresh income interval
    pub refresh_income_interval: Slot,
}

impl Registry {
    /// Initialize a voting pool
    pub fn init(manager: Pubkey) -> Registry {
        let mut r = Registry::default();
        r.account_type = AccountType::Registry;
        r.manager = manager;

        r
    }
}

impl Sealed for Registry {}
impl Pack for Registry {
    const LEN: usize =
        1 + (32 + 32 + 32 + 32 + (32 * TOTAL_DISTRIBUTIONS) + (32 * TOTAL_DISTRIBUTIONS) + 8);

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

impl IsInitialized for Registry {
    fn is_initialized(&self) -> bool {
        self.account_type == AccountType::Registry
    }
}

impl Uninitialized for Registry {
    fn is_uninitialized(&self) -> bool {
        self.account_type == AccountType::default()
    }
}
