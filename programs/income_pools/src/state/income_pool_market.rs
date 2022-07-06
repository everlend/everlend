//! Income pool market state definitions
use super::*;
use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use everlend_utils::Uninitialized;
use solana_program::{
    msg,
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};

/// Income pool market
#[repr(C)]
#[derive(Debug, BorshDeserialize, BorshSerialize, BorshSchema, Default)]
pub struct IncomePoolMarket {
    /// Account type - IncomePoolMarket
    pub account_type: AccountType,
    /// Market manager
    pub manager: Pubkey,
    /// General pool market
    pub general_pool_market: Pubkey,
}

impl IncomePoolMarket {
    /// Initialize a income pool market
    pub fn init(&mut self, params: InitIncomePoolMarketParams) {
        self.account_type = AccountType::IncomePoolMarket;
        self.manager = params.manager;
        self.general_pool_market = params.general_pool_market;
    }
}

/// Initialize a income pool market params
pub struct InitIncomePoolMarketParams {
    /// Market manager
    pub manager: Pubkey,
    /// General pool market
    pub general_pool_market: Pubkey,
}

impl Sealed for IncomePoolMarket {}
impl Pack for IncomePoolMarket {
    // 1 + 32 + 32
    const LEN: usize = 65;

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let mut slice = dst;
        self.serialize(&mut slice).unwrap()
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, solana_program::program_error::ProgramError> {
        Self::try_from_slice(src).map_err(|_| {
            msg!("Failed to deserialize");
            msg!("Actual LEN: {}", std::mem::size_of::<IncomePoolMarket>());
            ProgramError::InvalidAccountData
        })
    }
}

impl IsInitialized for IncomePoolMarket {
    fn is_initialized(&self) -> bool {
        self.account_type == AccountType::IncomePoolMarket
    }
}

impl Uninitialized for IncomePoolMarket {
    fn is_uninitialized(&self) -> bool {
        self.account_type == AccountType::default()
    }
}
