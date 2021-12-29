use crate::EverlendError;
use solana_program::program_error::ProgramError;

/// Multiply precision
pub const PRECISION_MUL: u128 = 1_000_000_000;

pub fn abs_diff(a: u64, b: u64) -> Result<u64, ProgramError> {
    let diff = (a as i128)
        .checked_sub(b as i128)
        .ok_or(EverlendError::MathOverflow)?
        .checked_abs()
        .ok_or(EverlendError::MathOverflow)?;

    Ok(diff as u64)
}

pub fn percent_div(a: u64, b: u64) -> Result<u64, ProgramError> {
    let res = (a as u128)
        .checked_mul(PRECISION_MUL)
        .ok_or(EverlendError::MathOverflow)?
        .checked_div(b as u128)
        .ok_or(EverlendError::MathOverflow)?;

    Ok(res as u64)
}

pub fn amount_share(total_amount: u64, percent: u64) -> Result<u64, ProgramError> {
    let amount = (percent as u128)
        .checked_mul(total_amount as u128)
        .ok_or(EverlendError::MathOverflow)?
        .checked_div(PRECISION_MUL)
        .ok_or(EverlendError::MathOverflow)?;

    Ok(amount as u64)
}

/// Convert the UI representation of a bp (like 0.5) to the raw bp
pub fn ui_bp_to_bp(ui_ratio: f64) -> u16 {
    (ui_ratio * 10_000f64).round() as u16
}
