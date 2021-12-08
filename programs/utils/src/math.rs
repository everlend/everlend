use crate::EverlendError;
use solana_program::program_error::ProgramError;

/// Multiply precision
pub const PRECISION_MUL: u128 = 1_000_000_000;

pub fn amount_percent_diff(
    percent_greater: u64,
    percent_lesser: u64,
    total_amount: u64,
) -> Result<u64, ProgramError> {
    let diff = (percent_greater as u128)
        .checked_sub(percent_lesser as u128)
        .ok_or(EverlendError::MathOverflow)?;

    let amount = diff
        .checked_mul(total_amount as u128)
        .ok_or(EverlendError::MathOverflow)?
        .checked_div(PRECISION_MUL)
        .ok_or(EverlendError::MathOverflow)?;

    Ok(amount as u64)
}
