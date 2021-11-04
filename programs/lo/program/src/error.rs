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
pub enum LiquidityOracleError {
    /// Oracle program already initialized
    #[error("Oracle already initialized")]
    AlreadyInitialized,
}

impl PrintProgramError for LiquidityOracleError {
    fn print<E>(&self) {
        msg!("Error: {}", &self.to_string());
    }
}

impl From<LiquidityOracleError> for ProgramError {
    fn from(e: LiquidityOracleError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

impl<T> DecodeError<T> for LiquidityOracleError {
    fn type_of() -> &'static str {
        "LiquidityOracleError"
    }
}
