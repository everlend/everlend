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
pub enum DepositorError {
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

    /// Invaliud instruction order
    #[error("Invalid instruction order")]
    InvalidInstructionOrder,

    /// Incorrect instruction program id
    #[error("Incorrect instruction program id")]
    IncorrectInstructionProgramId,

    /// Wrong instruction amount
    #[error("Wrong instruction amount")]
    WrongInstructionAmount,
}

impl PrintProgramError for DepositorError {
    fn print<E>(&self) {
        msg!("Error: {}", &self.to_string());
    }
}

impl From<DepositorError> for ProgramError {
    fn from(e: DepositorError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

impl<T> DecodeError<T> for DepositorError {
    fn type_of() -> &'static str {
        "DepositorError"
    }
}
