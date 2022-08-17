use crate::EverlendError;
use solana_program::program_error::ProgramError;
use spl_math::precise_number::PreciseNumber;

/// Scale for precision
pub const PRECISION_SCALER: u128 = 1_000_000_000;

pub fn abs_diff(a: u64, b: u64) -> Result<u64, ProgramError> {
    let res = (a as i128)
        .checked_sub(b as i128)
        .ok_or(EverlendError::MathOverflow)?
        .checked_abs()
        .ok_or(EverlendError::MathOverflow)?;

    Ok(res as u64)
}

pub fn percent_ratio(amount: u64, total: u64, collateral_amount: u64) -> Result<u64, ProgramError> {
    if total == 0 {
        return Ok(0);
    }

    let amount = PreciseNumber::new(amount.into()).ok_or(EverlendError::MathOverflow)?;
    let total = PreciseNumber::new(total.into()).ok_or(EverlendError::MathOverflow)?;

    let percentage = amount
        .checked_div(&total)
        .ok_or(EverlendError::MathOverflow)?;

    let collateral_amount = PreciseNumber::new(collateral_amount.into())
        .ok_or(EverlendError::MathOverflow)?
        .checked_mul(&percentage)
        .ok_or(EverlendError::MathOverflow)?
        .to_imprecise()
        .ok_or(EverlendError::MathOverflow)?;

    Ok(collateral_amount as u64)
}

pub fn share_floor(amount: u64, percent: u64) -> Result<u64, ProgramError> {
    let res = (percent as u128)
        .checked_mul(amount as u128)
        .ok_or(EverlendError::MathOverflow)?
        .checked_div(PRECISION_SCALER)
        .ok_or(EverlendError::MathOverflow)?;

    Ok(res as u64)
}
