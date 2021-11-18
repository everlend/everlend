//! Token distribution state definitions.

use super::AccountType;
use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::{
    clock::Slot,
    msg,
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};

pub const LENDINGS_SIZE: usize = 10;

pub type DistributionArray = [LiquidityDistribution; LENDINGS_SIZE];

#[repr(C)]
#[derive(Debug, Clone, BorshDeserialize, BorshSerialize, BorshSchema, PartialEq, Default)]
pub struct TokenDistribution {
    // Account type.
    pub account_type: AccountType,
    //Last update slot
    pub slot: Slot, //u64 Len 8
    pub distribution: DistributionArray,
}

#[derive(Debug, Clone, Copy, BorshDeserialize, BorshSerialize, BorshSchema, PartialEq, Default)]
pub struct LiquidityDistribution {
    pub money_market: Pubkey, //[u8, 32] Len 32
    pub percent: f64,         //f64 Len 8
}

impl TokenDistribution {
    /// Initialize a liquidity oracle.
    pub fn init(&mut self) {
        self.account_type = AccountType::TokenDistribution;
    }

    /// Update a liquidity oracle token distribution
    pub fn update(&mut self, slot: Slot, distribution: DistributionArray) {
        self.slot = slot;
        self.distribution = distribution;
    }
}

impl Sealed for TokenDistribution {}

impl Pack for TokenDistribution {
    // Enum + Slot size + LDistribution size * LENDINGS_SIZE
    const LEN: usize = 1 + 8 + (40 * LENDINGS_SIZE);

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let mut slice = dst;
        self.serialize(&mut slice).unwrap()
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        Self::try_from_slice(src).map_err(|_| {
            msg!("Failed to deserialize");
            msg!(
                "Actual LEN: {}",
                std::mem::size_of::<TokenDistribution>()
            );
            ProgramError::InvalidAccountData
        })
    }
}

impl IsInitialized for TokenDistribution {
    fn is_initialized(&self) -> bool {
        self.account_type != AccountType::Uninitialized
            && self.account_type == AccountType::TokenDistribution
    }
}
