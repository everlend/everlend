//! Error types

use num_derive::FromPrimitive;
use solana_program::{
    decode_error::DecodeError,
    msg,
    program_error::{PrintProgramError, ProgramError},
};
use thiserror::Error;

/// Errors that may be returned by the program.
#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum EverlendError {
    /// 0
    /// Input account owner
    #[error("Input account owner")]
    InvalidAccountOwner,

    /// Math operation overflow
    #[error("Math operation overflow")]
    MathOverflow,

    /// Data type mismatch
    #[error("Data type mismatch")]
    DataTypeMismatch,

    /// Ammount allowed of interest on the borrowing is exceeded
    #[error("Ammount allowed of interest on the borrowing is exceeded")]
    AmountAllowedCheckFailed,

    /// Amount borrowed less then repay amount
    #[error("Amount allowed of interest on the borrowing is exceeded")]
    RepayAmountCheckFailed,

    /// 5
    /// Incorrect instruction program id
    #[error("Incorrect instruction program id")]
    IncorrectInstructionProgramId,

    /// Rebalancing

    /// Incomplete rebalancing
    #[error("Incomplete rebalancing")]
    IncompleteRebalancing,

    /// Rebalancing is completed
    #[error("Rebalancing is completed")]
    RebalancingIsCompleted,

    /// Money market does not match
    #[error("Rebalancing: Money market does not match")]
    InvalidRebalancingMoneyMarket,

    /// Operation does not match
    #[error("Rebalancing: Operation does not match")]
    InvalidRebalancingOperation,

    /// 10
    /// Amount does not match
    #[error("Rebalancing: Amount does not match")]
    InvalidRebalancingAmount,

    /// Token distribution is stale
    #[error("Rebalancing: Token distribution is stale")]
    TokenDistributionIsStale,

    /// Income has already been refreshed recently
    #[error("Rebalancing: Income has already been refreshed recently")]
    IncomeRefreshed,

    /// Withdraw requests

    /// Invalid ticket
    #[error("Withdraw requests: Invalid ticket")]
    WithdrawRequestsInvalidTicket,

    /// Temporary unavailable for migration
    #[error("Instruction temporary unavailable")]
    TemporaryUnavailable,
}

impl PrintProgramError for EverlendError {
    fn print<E>(&self) {
        msg!("Error: {}", &self.to_string());
    }
}

impl From<EverlendError> for ProgramError {
    fn from(e: EverlendError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

impl<T> DecodeError<T> for EverlendError {
    fn type_of() -> &'static str {
        "EverlendError"
    }
}
