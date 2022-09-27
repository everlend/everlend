//! Rebalancing step state definitions

use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::{
    clock::Slot,
    msg,
    program_error::ProgramError,
    program_pack::{Pack, Sealed},
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
    /// Refresh withdraw
    RefreshWithdraw,
    /// Refresh deposit
    RefreshDeposit,
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
    /// Money market index
    pub money_market_index: u8,

    /// Deposit or withdraw
    pub operation: RebalancingOperation,

    /// Liquidity amount
    pub liquidity_amount: u64,

    /// Collateral amount (Undefined for deposit)
    pub collateral_amount: Option<u64>,

    /// Slot when executed deposit or withdraw
    pub executed_at: Option<Slot>,
}

impl RebalancingStep {
    /// Constructor
    pub fn new(
        money_market_index: u8,
        operation: RebalancingOperation,
        liquidity_amount: u64,
        collateral_amount: Option<u64>,
    ) -> Self {
        RebalancingStep {
            money_market_index,
            operation,
            liquidity_amount,
            collateral_amount,
            executed_at: None,
        }
    }

    /// Execute operation
    pub fn set_executed_at(&mut self, slot: Slot) {
        self.executed_at = Some(slot);
    }
}

impl Sealed for RebalancingStep {}
impl Pack for RebalancingStep {
    // 1 + 1 + 8 + (1 + 8) + (1 + 8)
    const LEN: usize = 28;

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
