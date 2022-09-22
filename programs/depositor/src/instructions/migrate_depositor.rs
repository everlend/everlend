use everlend_utils::EverlendError;
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, program_error::ProgramError,
    pubkey::Pubkey,
};
use std::{iter::Enumerate, slice::Iter};

/// Instruction context
pub struct MigrateDepositorContext {}

impl<'a, 'b> MigrateDepositorContext {
    /// New MigrateDepositor instruction context
    pub fn new(
        _program_id: &Pubkey,
        _account_info_iter: &mut Enumerate<Iter<'a, AccountInfo<'b>>>,
    ) -> Result<MigrateDepositorContext, ProgramError> {
        Err(EverlendError::TemporaryUnavailable.into())
    }

    /// Process MigrateDepositor instruction
    pub fn process(
        &self,
        _program_id: &Pubkey,
        _account_info_iter: &'a mut Enumerate<Iter<'a, AccountInfo<'b>>>,
    ) -> ProgramResult {
        Ok(())
    }
}
