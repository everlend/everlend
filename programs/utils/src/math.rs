use crate::EverlendError;
use solana_program::program_error::ProgramError;

/// Scale for precision
pub const PRECISION_SCALER: u128 = 1_000_000_000;

pub fn absdiff(a: u64, b: u64) -> Result<u64, ProgramError> {
    let res = (a as i128)
        .checked_sub(b as i128)
        .ok_or(EverlendError::MathOverflow)?
        .checked_abs()
        .ok_or(EverlendError::MathOverflow)?;

    Ok(res as u64)
}

pub fn percent_ratio(a: u64, b: u64) -> Result<u64, ProgramError> {
    if b == 0 {
        return Ok(0);
    }

    let res = (a as u128)
        .checked_mul(PRECISION_SCALER)
        .ok_or(EverlendError::MathOverflow)?
        .checked_div(b as u128)
        .ok_or(EverlendError::MathOverflow)?;

    Ok(res as u64)
}

pub fn share(amount: u64, percent: u64) -> Result<u64, ProgramError> {
    let res = (percent as u128)
        .checked_mul(amount as u128)
        .ok_or(EverlendError::MathOverflow)?
        .checked_div(PRECISION_SCALER)
        .ok_or(EverlendError::MathOverflow)?;

    Ok(res as u64)
}
