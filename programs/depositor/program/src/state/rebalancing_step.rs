//! Rebalancing step state definitions

use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use everlend_utils::EverlendError;
use solana_program::{
    clock::Slot,
    msg,
    program_error::ProgramError,
    program_pack::{Pack, Sealed},
    pubkey::Pubkey,
};

/// Enum representing rebalancing step type operation
#[derive(
    Clone, Copy, Debug, PartialEq, PartialOrd, BorshDeserialize, BorshSerialize, BorshSchema,
)]
pub enum RebalancingOperation {
    /// Withdraw
    Withdraw,
    /// Deposit
    Deposit,
}

impl Default for RebalancingOperation {
    fn default() -> Self {
        RebalancingOperation::Deposit
    }
}

/// RebalancingStep
#[repr(C)]
#[derive(Debug, Clone, Copy, BorshDeserialize, BorshSerialize, BorshSchema, PartialEq, Default)]
pub struct RebalancingStep {
    /// Money market program id
    pub money_market_program_id: Pubkey,

    /// Deposit or withdraw
    pub operation: RebalancingOperation,

    /// Amount
    pub amount: u64,

    /// Slot when executed deposit or withdraw
    pub executed_at: Option<Slot>,
}

impl RebalancingStep {
    /// Constructor
    pub fn new(
        money_market_program_id: Pubkey,
        operation: RebalancingOperation,
        amount: u64,
    ) -> Self {
        RebalancingStep {
            money_market_program_id,
            operation,
            amount,
            executed_at: None,
        }
    }

    /// Execute operation
    pub fn execute(
        &mut self,
        money_market_program_id: Pubkey,
        operation: RebalancingOperation,
        amount: u64,
        slot: Slot,
    ) -> Result<(), ProgramError> {
        if self.money_market_program_id != money_market_program_id {
            return Err(EverlendError::InvalidRebalancingMoneyMarket.into());
        }

        if self.operation != operation {
            return Err(EverlendError::InvalidRebalancingOperation.into());
        }

        if self.amount != amount {
            return Err(EverlendError::InvalidRebalancingAmount.into());
        }

        self.executed_at = Some(slot);
        Ok(())
    }
}

impl Sealed for RebalancingStep {}
impl Pack for RebalancingStep {
    // 32 + 1 + 8 + (1 + 8)
    const LEN: usize = 50;

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let mut slice = dst;
        self.serialize(&mut slice).unwrap()
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        Self::try_from_slice(src).map_err(|_| {
            msg!("Failed to deserialize");
            msg!("Actual LEN: {}", std::mem::size_of::<RebalancingStep>());
            ProgramError::InvalidAccountData
        })
    }
}
